//! Clipboard history extension process entry point.

use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use nanika_extension_clipboard::{
    CLEAR_ACTION_ID, CLIPBOARD_PAGE_SIZE, COPY_ACTION_ID, ClipboardConfig, ClipboardEntry,
    ClipboardMonitor, ClipboardViewState, ClipboardWorker, FILE_COLLECTION_PREVIEW_LIMIT,
    FileIconWorker, RuntimePaths, VIEW_ID, matching_entries, render_clipboard_view,
};
use nanika_protocol::{
    ClipboardContent, HostServiceRequest, HostServiceResponse, Message, NavigationEffect,
    PROTOCOL_NAME, ViewEvent, read_frame, write_frame,
};

type SharedOutput = Arc<Mutex<BufWriter<std::io::Stdout>>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = RuntimePaths::parse(std::env::args().skip(1))?;
    let mut input = BufReader::new(stdin().lock());
    let output = Arc::new(Mutex::new(BufWriter::new(stdout())));
    let (initialize_request_id, configuration) = match read_frame(&mut input)? {
        Some(Message::Initialize {
            request_id,
            protocol,
            configuration,
        }) if protocol == PROTOCOL_NAME => (request_id, configuration),
        Some(Message::Initialize { request_id, .. }) => {
            send_error(
                &output,
                Some(request_id),
                "unsupported_protocol",
                "the requested extension protocol is unsupported",
            )?;
            return Ok(());
        }
        Some(message) => {
            send_error(
                &output,
                request_id(&message),
                "not_initialized",
                "initialize must be the first request",
            )?;
            return Ok(());
        }
        None => return Ok(()),
    };
    let config = match ClipboardConfig::from_configuration(&configuration) {
        Ok(config) => config,
        Err(message) => {
            send_error(
                &output,
                Some(initialize_request_id),
                "invalid_configuration",
                &message,
            )?;
            return Ok(());
        }
    };
    let entries = Arc::new(RwLock::new(Vec::<ClipboardEntry>::new()));
    let notifications_enabled = Arc::new(AtomicBool::new(true));
    let callback_enabled = Arc::clone(&notifications_enabled);
    let invalidation_output = Arc::clone(&output);
    let view_invalidated: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        if !callback_enabled.load(Ordering::Acquire) {
            return;
        }
        if let Err(error) = send_frame(
            &invalidation_output,
            &Message::ViewInvalidated {
                view_id: VIEW_ID.to_owned(),
            },
        ) {
            eprintln!("clipboard view invalidation failed: {error}");
        }
    });
    let worker = ClipboardWorker::spawn(
        paths.database_path(),
        paths.payload_root(),
        config,
        Arc::clone(&entries),
        Arc::clone(&view_invalidated),
    )?;
    send_frame(
        &output,
        &Message::Initialized {
            request_id: initialize_request_id,
            protocol: PROTOCOL_NAME.to_owned(),
        },
    )?;
    let monitor = ClipboardMonitor::spawn(&worker)?;
    let icon_root = paths
        .cache_root
        .join("icons")
        .join(nanika_extension_clipboard::EXTENSION_ID);
    let icon_worker = FileIconWorker::spawn(icon_root, view_invalidated)?;
    let mut view_state = None;
    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Query {
                request_id,
                generation,
                ..
            } => match worker.last_error() {
                Some(message) => send_error(
                    &output,
                    Some(request_id),
                    "clipboard_worker_failed",
                    &message,
                )?,
                None => send_frame(
                    &output,
                    &Message::Snapshot {
                        request_id,
                        generation,
                        complete: true,
                        entries: Vec::new(),
                    },
                )?,
            },
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id,
            } => {
                if entry_id != VIEW_ID || action_id != nanika_protocol::VIEW_OPEN_ACTION_ID {
                    send_error(
                        &output,
                        Some(request_id),
                        "unknown_action",
                        "clipboard view or action does not exist",
                    )?;
                    continue;
                }
                let mut state = ClipboardViewState::new();
                // First paint reads in-memory results only. Filesystem metadata, persistent
                // cache lookup, and native acquisition all stay on the icon worker.
                let view = render_clipboard_view(&mut state, &entries, &|path| {
                    icon_worker.resolution(path)
                });
                view_state = Some(state);
                send_frame(
                    &output,
                    &Message::Result {
                        request_id,
                        generation,
                        effect: NavigationEffect::Push {
                            view_id: VIEW_ID.to_owned(),
                            revision: 1,
                            view: Box::new(view),
                        },
                    },
                )?;
                if let Some(state) = &view_state {
                    schedule_visible_icons(&icon_worker, state, &entries);
                }
            }
            Message::ViewEvent {
                request_id,
                generation,
                view_id,
                revision,
                event,
            } if view_id == VIEW_ID => handle_view_event(
                &mut input,
                &output,
                &worker,
                &monitor,
                &icon_worker,
                &entries,
                &mut view_state,
                request_id,
                generation,
                revision,
                event,
            )?,
            Message::ViewClose {
                request_id,
                view_id,
            } if view_id == VIEW_ID => {
                view_state = None;
                send_frame(
                    &output,
                    &Message::ViewClosed {
                        request_id,
                        view_id,
                    },
                )?;
            }
            Message::Refresh {
                request_id,
                generation,
            } => send_frame(
                &output,
                &Message::Refreshed {
                    request_id,
                    generation,
                },
            )?,
            Message::Cancel { .. } => {}
            Message::PrepareEntries { .. } => {}
            Message::ConfigurationChanged {
                request_id,
                configuration,
            } => match ClipboardConfig::from_configuration(&configuration)
                .and_then(|config| worker.apply_retention(config))
            {
                Ok(()) => send_frame(&output, &Message::ConfigurationApplied { request_id })?,
                Err(message) => send_error(
                    &output,
                    Some(request_id),
                    "configuration_apply_failed",
                    &message,
                )?,
            },
            Message::Shutdown { request_id } => {
                notifications_enabled.store(false, Ordering::Release);
                send_frame(&output, &Message::ShutdownAck { request_id })?;
                break;
            }
            message => send_error(
                &output,
                request_id(&message),
                "unsupported_message",
                "the clipboard extension received an unsupported message",
            )?,
        }
    }
    monitor.shutdown();
    icon_worker.shutdown();
    worker.shutdown();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_view_event(
    input: &mut impl std::io::Read,
    output: &SharedOutput,
    worker: &ClipboardWorker,
    monitor: &ClipboardMonitor,
    icon_worker: &FileIconWorker,
    entries: &Arc<RwLock<Vec<ClipboardEntry>>>,
    view_state: &mut Option<ClipboardViewState>,
    request_id: String,
    generation: u64,
    revision: u64,
    event: ViewEvent,
) -> Result<(), nanika_protocol::FrameError> {
    let Some(state) = view_state.as_mut() else {
        return send_error(
            output,
            Some(request_id),
            "unknown_view",
            "clipboard view is not open",
        );
    };
    if state.revision != revision {
        return send_error(
            output,
            Some(request_id),
            "stale_view",
            "clipboard view revision is stale",
        );
    }
    match event {
        ViewEvent::Resumed => {}
        ViewEvent::Invalidated => {}
        ViewEvent::ActionInvoked { action_id, .. } if action_id == CLEAR_ACTION_ID => {
            // Freeze the matching IDs before queueing; later captures are outside this request.
            let entry_ids = {
                let current = entries.read().unwrap_or_else(|error| error.into_inner());
                matching_entries(state, &current)
                    .into_iter()
                    .map(|entry| entry.entry_id.clone())
                    .collect()
            };
            if let Err(message) = worker.clear(entry_ids) {
                return send_error(
                    output,
                    Some(request_id),
                    "clipboard_history_clear_failed",
                    &message,
                );
            }
            state.selected_item_id = None;
        }
        ViewEvent::SearchChanged { text } => {
            state.query = text;
            state.visible_limit = CLIPBOARD_PAGE_SIZE;
        }
        ViewEvent::SelectionChanged { item_id } => state.selected_item_id = item_id,
        ViewEvent::FilterChanged { filter_id, value }
            if filter_id == "contentType"
                && matches!(value.as_str(), "all" | "text" | "files" | "images") =>
        {
            state.content_type = value;
            state.visible_limit = CLIPBOARD_PAGE_SIZE;
        }
        ViewEvent::LoadMore { cursor } if cursor == state.visible_limit.to_string() => {
            state.visible_limit = state.visible_limit.saturating_add(CLIPBOARD_PAGE_SIZE);
        }
        ViewEvent::ActionInvoked { item_id, action_id } if action_id == COPY_ACTION_ID => {
            let content = item_id.as_deref().and_then(|item_id| {
                entries
                    .read()
                    .unwrap_or_else(|error| error.into_inner())
                    .iter()
                    .find(|entry| entry.entry_id == item_id)
                    .map(|entry| entry.content.clone())
            });
            let Some(content) = content else {
                return send_error(
                    output,
                    Some(request_id),
                    "unknown_action",
                    "clipboard entry or action does not exist",
                );
            };
            monitor.begin_internal_write();
            let copied = match write_clipboard(input, output, &request_id, generation, content) {
                Ok(copied) => copied,
                Err(error) => {
                    if let Err(message) = monitor.cancel_internal_write() {
                        eprintln!("clipboard capture recovery failed: {message}");
                    }
                    return Err(error);
                }
            };
            if let Some(revision) = copied {
                if let Err(message) = monitor.complete_internal_write(revision) {
                    return send_error(
                        output,
                        Some(request_id),
                        "clipboard_monitor_failed",
                        &message,
                    );
                }
                send_frame(
                    output,
                    &Message::ViewUpdated {
                        request_id,
                        generation,
                        view_id: VIEW_ID.to_owned(),
                        revision: state.revision,
                        effect: NavigationEffect::Dismiss,
                        view: None,
                    },
                )?;
            } else {
                if let Err(message) = monitor.cancel_internal_write() {
                    eprintln!("clipboard capture recovery failed: {message}");
                }
            }
            return Ok(());
        }
        _ => {
            return send_error(
                output,
                Some(request_id),
                "invalid_view_event",
                "clipboard view event is invalid",
            );
        }
    }
    state.revision = state.revision.saturating_add(1);
    schedule_visible_icons(icon_worker, state, entries);
    let view = render_clipboard_view(state, entries, &|path| icon_worker.resolution(path));
    send_frame(
        output,
        &Message::ViewUpdated {
            request_id,
            generation,
            view_id: VIEW_ID.to_owned(),
            revision: state.revision,
            effect: NavigationEffect::None,
            view: Some(view),
        },
    )
}

fn write_clipboard(
    input: &mut impl std::io::Read,
    output: &SharedOutput,
    request_id: &str,
    generation: u64,
    content: ClipboardContent,
) -> Result<Option<u64>, nanika_protocol::FrameError> {
    let service_request_id = format!("host-{request_id}");
    send_frame(
        output,
        &Message::HostRequest {
            request_id: service_request_id.clone(),
            parent_request_id: request_id.to_owned(),
            generation,
            request: HostServiceRequest::WriteClipboard { content },
        },
    )?;
    loop {
        match read_frame(input)? {
            Some(Message::HostResponse {
                request_id: response_id,
                parent_request_id,
                generation: response_generation,
                response: HostServiceResponse::ClipboardWritten { revision },
            }) if response_id == service_request_id
                && parent_request_id == request_id
                && response_generation == generation =>
            {
                return Ok(Some(revision));
            }
            Some(Message::Error {
                request_id: Some(response_id),
                code,
                message,
            }) if response_id == service_request_id => {
                send_error(output, Some(request_id.to_owned()), &code, &message)?;
                return Ok(None);
            }
            Some(_) => {}
            None => return Ok(None),
        }
    }
}

fn send_error(
    output: &SharedOutput,
    request_id: Option<String>,
    code: &str,
    message: &str,
) -> Result<(), nanika_protocol::FrameError> {
    send_frame(
        output,
        &Message::Error {
            request_id,
            code: code.to_owned(),
            message: message.to_owned(),
        },
    )
}

fn send_frame(output: &SharedOutput, message: &Message) -> Result<(), nanika_protocol::FrameError> {
    write_frame(
        &mut *output.lock().unwrap_or_else(|error| error.into_inner()),
        message,
    )
}

fn schedule_visible_icons(
    worker: &FileIconWorker,
    state: &ClipboardViewState,
    entries: &RwLock<Vec<ClipboardEntry>>,
) {
    let entries = entries.read().unwrap_or_else(|error| error.into_inner());
    let visible = matching_entries(state, &entries)
        .into_iter()
        .take(state.visible_limit)
        .collect::<Vec<_>>();
    let selected = state.selected_item_id.as_deref();
    let selected_paths = visible
        .iter()
        .filter(|entry| Some(entry.entry_id.as_str()) == selected)
        .flat_map(|entry| match &entry.content {
            ClipboardContent::Files { paths } => {
                paths[..paths.len().min(FILE_COLLECTION_PREVIEW_LIMIT)].iter()
            }
            _ => [].iter(),
        });
    let other_paths = visible
        .iter()
        .filter(|entry| Some(entry.entry_id.as_str()) != selected)
        .filter_map(|entry| match &entry.content {
            ClipboardContent::Files { paths } => paths.first(),
            _ => None,
        });
    // Multi-file detail renders a bounded icon stack. Rows need only the first file icon, so
    // scheduling remains proportional to the visible page rather than payload size.
    worker.schedule(
        selected_paths
            .chain(other_paths)
            .map(std::path::PathBuf::from),
    );
}

fn request_id(message: &Message) -> Option<String> {
    match message {
        Message::Initialize { request_id, .. }
        | Message::Initialized { request_id, .. }
        | Message::Query { request_id, .. }
        | Message::Snapshot { request_id, .. }
        | Message::Invoke { request_id, .. }
        | Message::Result { request_id, .. }
        | Message::ViewEvent { request_id, .. }
        | Message::ViewUpdated { request_id, .. }
        | Message::ViewClose { request_id, .. }
        | Message::ViewClosed { request_id, .. }
        | Message::Cancel { request_id, .. }
        | Message::Refresh { request_id, .. }
        | Message::Refreshed { request_id, .. }
        | Message::ConfigurationChanged { request_id, .. }
        | Message::ConfigurationApplied { request_id }
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. }
        | Message::Shutdown { request_id }
        | Message::ShutdownAck { request_id } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
        Message::CandidatesChanged
        | Message::ViewInvalidated { .. }
        | Message::PrepareEntries { .. } => None,
    }
}

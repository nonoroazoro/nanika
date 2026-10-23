use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use nanika_extension_clipboard::{
    CLEAR_ACTION_ID, COPY_ACTION_ID, ClipboardConfig, ClipboardMonitor, ClipboardPresentation,
    ClipboardViewState, ClipboardWorker, FileIconWorker, RuntimePaths, VIEW_ID
};
use nanika_protocol::{
    ClipboardContent, HostServiceRequest, HostServiceResponse, Message, NavigationEffect,
    PROTOCOL_NAME, ViewEvent, read_frame, write_frame
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
            configuration
        }) if protocol == PROTOCOL_NAME => (request_id, configuration),
        Some(Message::Initialize { request_id, .. }) => {
            _send_error(
                &output,
                Some(request_id),
                "unsupported_protocol",
                "error-unsupported-protocol",
                "the requested extension protocol is unsupported"
            )?;
            return Ok(());
        }
        Some(message) => {
            _send_error(
                &output,
                request_id(&message),
                "not_initialized",
                "error-not-initialized",
                "initialize must be the first request"
            )?;
            return Ok(());
        }
        None => return Ok(())
    };
    let config = match ClipboardConfig::from_configuration(&configuration) {
        Ok(config) => config,
        Err(message) => {
            _send_error(
                &output,
                Some(initialize_request_id),
                "invalid_configuration",
                "error-invalid-configuration",
                &message
            )?;
            return Ok(());
        }
    };
    let notifications_enabled = Arc::new(AtomicBool::new(true));
    let callback_enabled = Arc::clone(&notifications_enabled);
    let invalidation_output = Arc::clone(&output);
    let view_invalidated: Arc<dyn Fn(nanika_protocol::TextInvalidation) + Send + Sync> =
        Arc::new(move |text| {
            if !callback_enabled.load(Ordering::Acquire) {
                return;
            }
            if let Err(error) = send_frame(&invalidation_output, &Message::ViewsChanged { text }) {
                eprintln!("clipboard view invalidation failed: {error}");
            }
        });
    let worker = ClipboardWorker::spawn(
        paths.database_path(),
        paths.payload_root(),
        config,
        Arc::clone(&view_invalidated)
    )?;
    send_frame(
        &output,
        &Message::Initialized {
            request_id: initialize_request_id,
            protocol: PROTOCOL_NAME.to_owned()
        }
    )?;
    let monitor = ClipboardMonitor::spawn(&worker)?;
    let icon_root = paths
        .cache_root
        .join("icons")
        .join(nanika_extension_clipboard::EXTENSION_ID);
    let icon_worker = FileIconWorker::spawn(
        icon_root,
        Arc::new(move || view_invalidated(Default::default()))
    )?;
    let mut view_state = None;
    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Query {
                request_id,
                generation,
                ..
            } => send_frame(
                &output,
                &Message::Snapshot {
                    replace: true,
                    removed: Vec::new(),
                    request_id,
                    generation,
                    complete: true,
                    entries: Vec::new()
                }
            )?,
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id
            } if entry_id == nanika_protocol::SETTINGS_ENTRY_ID && action_id == CLEAR_ACTION_ID => {
                match worker.clear_all() {
                    Ok(()) => send_frame(
                        &output,
                        &Message::Result {
                            request_id,
                            generation,
                            effect: NavigationEffect::None
                        }
                    )?,
                    Err(error) => _send_error(
                        &output,
                        Some(request_id),
                        "clipboard_history_clear_failed",
                        "error-clear-history",
                        &error
                    )?
                }
            }
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id
            } => {
                if entry_id != VIEW_ID || action_id != nanika_protocol::VIEW_OPEN_ACTION_ID {
                    _send_error(
                        &output,
                        Some(request_id),
                        "unknown_action",
                        "error-action-unavailable",
                        "clipboard view or action does not exist"
                    )?;
                    continue;
                }
                let mut presentation = match worker.present(ClipboardViewState::new(), None) {
                    Ok(presentation) => presentation,
                    Err(error) => {
                        _send_error(
                            &output,
                            Some(request_id),
                            "view_failed",
                            "error-open-history",
                            &error
                        )?;
                        continue;
                    }
                };
                presentation.decorate_icons(&|path| icon_worker.resolution(path));
                if let Err(message) = presentation.view.validate() {
                    _send_error(
                        &output,
                        Some(request_id),
                        "view_failed",
                        "error-open-history",
                        &message
                    )?;
                    continue;
                }
                send_frame(
                    &output,
                    &Message::Result {
                        request_id,
                        generation,
                        effect: NavigationEffect::Push {
                            view_id: VIEW_ID.to_owned(),
                            revision: 1,
                            view: Box::new(presentation.view.clone())
                        }
                    }
                )?;
                icon_worker.schedule(presentation.icon_paths());
                view_state = Some(presentation);
            }
            Message::ViewEvent {
                request_id,
                generation,
                view_id,
                revision,
                event
            } if view_id == VIEW_ID => handle_view_event(
                &mut input,
                &output,
                &worker,
                &monitor,
                &icon_worker,
                &mut view_state,
                request_id,
                generation,
                revision,
                event
            )?,
            Message::TextRead {
                request_id,
                generation,
                view_id,
                read
            } if view_id == VIEW_ID && view_state.is_some() => match worker.read_text(&read) {
                Ok(chunk) => send_frame(
                    &output,
                    &Message::TextReadResult {
                        request_id,
                        generation,
                        chunk
                    }
                )?,
                Err(error) => send_frame(
                    &output,
                    &Message::Error {
                        request_id: Some(request_id),
                        error: nanika_protocol::ProtocolError::new(
                            "text_read_failed",
                            nanika_protocol::message("error-text-read", [])
                        )
                        .with_detail(error)
                    }
                )?
            },
            Message::ViewClose {
                request_id,
                view_id
            } if view_id == VIEW_ID => {
                if let Err(message) = worker.close_view() {
                    _send_error(
                        &output,
                        Some(request_id),
                        "view_close_failed",
                        "error-close-history",
                        &message
                    )?;
                    continue;
                }
                view_state = None;
                send_frame(
                    &output,
                    &Message::ViewClosed {
                        request_id,
                        view_id
                    }
                )?;
            }
            Message::Refresh {
                request_id,
                generation
            } => send_frame(
                &output,
                &Message::Refreshed {
                    request_id,
                    generation
                }
            )?,
            Message::Cancel { .. } => {}
            Message::PrepareEntries { .. } => {}
            Message::ConfigurationChanged {
                request_id,
                configuration
            } => match ClipboardConfig::from_configuration(&configuration)
                .and_then(|config| worker.apply_configuration(config))
            {
                Ok(()) => send_frame(&output, &Message::ConfigurationApplied { request_id })?,
                Err(message) => _send_error(
                    &output,
                    Some(request_id),
                    "configuration_apply_failed",
                    "error-apply-configuration",
                    &message
                )?
            },
            message => _send_error(
                &output,
                request_id(&message),
                "unsupported_message",
                "error-unsupported-message",
                "the clipboard extension received an unsupported message"
            )?
        }
    }
    notifications_enabled.store(false, Ordering::Release);
    let errors = [
        monitor.shutdown(),
        icon_worker.shutdown(),
        worker.shutdown()
    ]
    .into_iter()
    .filter_map(Result::err)
    .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(std::io::Error::other(errors.join("; ")).into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_view_event(
    input: &mut impl std::io::Read,
    output: &SharedOutput,
    worker: &ClipboardWorker,
    monitor: &ClipboardMonitor,
    icon_worker: &FileIconWorker,
    view_state: &mut Option<ClipboardPresentation>,
    request_id: String,
    generation: u64,
    revision: u64,
    event: ViewEvent
) -> Result<(), nanika_protocol::FrameError> {
    let Some(current) = view_state.as_ref() else {
        return _send_error(
            output,
            Some(request_id),
            "unknown_view",
            "error-history-changed",
            "clipboard view is not open"
        );
    };
    if current.state.revision != revision {
        return _send_error(
            output,
            Some(request_id),
            "stale_view",
            "error-history-changed",
            "clipboard view revision is stale"
        );
    }
    let mut proposed = current.state.clone();
    let state = &mut proposed;
    let mut expected_collection_revision = None;
    match event {
        ViewEvent::Resumed => {}
        ViewEvent::Invalidated => {}
        ViewEvent::SearchChanged {
            text,
            minimum_items
        } => {
            state.query = text;
            state.reset_results(minimum_items);
        }
        ViewEvent::SelectionChanged {
            collection_id,
            index
        } => {
            match current.select_index(&collection_id, index) {
                Ok(selected) => *state = selected,
                Err(message) => {
                    return _send_error(
                        output,
                        Some(request_id),
                        "invalid_selection",
                        "error-history-interaction",
                        &message
                    );
                }
            }
            expected_collection_revision = Some(current.collection_revision);
        }
        ViewEvent::FilterChanged {
            filter_id,
            value,
            minimum_items
        } if filter_id == "contentType"
            && matches!(value.as_str(), "all" | "text" | "files" | "images") =>
        {
            state.content_type = value;
            state.reset_results(minimum_items);
        }
        ViewEvent::ListRangeChanged {
            collection_id,
            offset,
            count
        } => {
            if let Err(message) = nanika_extension_clipboard::read_range(
                state,
                &current.view,
                &collection_id,
                offset,
                count
            ) {
                return _send_error(
                    output,
                    Some(request_id),
                    "invalid_view_range",
                    "error-history-interaction",
                    &message
                );
            }
            expected_collection_revision = Some(current.collection_revision);
        }
        ViewEvent::ActionInvoked {
            item_id, action_id, ..
        } if action_id == COPY_ACTION_ID => {
            let content = match item_id
                .ok_or_else(|| "Clipboard item is missing.".to_owned())
                .and_then(|id| worker.content(id))
            {
                Ok(content) => content,
                Err(message) => {
                    return _send_error(
                        output,
                        Some(request_id),
                        "unknown_action",
                        "error-action-unavailable",
                        &message
                    );
                }
            };
            monitor.begin_internal_write();
            let copied = match write_clipboard(
                input,
                output,
                &request_id,
                generation,
                content.content().clone()
            ) {
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
                    return _send_error(
                        output,
                        Some(request_id),
                        "clipboard_monitor_failed",
                        "error-copied-monitor-failed",
                        &message
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
                        view: None
                    }
                )?;
            } else {
                if let Err(message) = monitor.cancel_internal_write() {
                    eprintln!("clipboard capture recovery failed: {message}");
                }
            }
            return Ok(());
        }
        _ => {
            return _send_error(
                output,
                Some(request_id),
                "invalid_view_event",
                "error-history-interaction",
                "clipboard view event is invalid"
            );
        }
    }
    let Some(next_revision) = state.revision.checked_add(1) else {
        return _send_error(
            output,
            Some(request_id),
            "view_revision_exhausted",
            "error-history-changed",
            "Reopen the view before continuing."
        );
    };
    state.revision = next_revision;
    let mut presentation = match worker.present(proposed, expected_collection_revision) {
        Ok(presentation) => presentation,
        Err(message) => {
            return _send_error(
                output,
                Some(request_id),
                "view_update_failed",
                "error-update-history",
                &message
            );
        }
    };
    presentation.decorate_icons(&|path| icon_worker.resolution(path));
    // Only a fully validated proposal that was sent becomes the protocol's current revision.
    if let Err(message) = presentation.view.validate() {
        return _send_error(
            output,
            Some(request_id),
            "view_update_failed",
            "error-update-history",
            &message
        );
    }
    send_frame(
        output,
        &Message::ViewUpdated {
            request_id,
            generation,
            view_id: VIEW_ID.to_owned(),
            revision: next_revision,
            effect: NavigationEffect::None,
            view: Some(presentation.view.clone())
        }
    )?;
    icon_worker.schedule(presentation.icon_paths());
    *view_state = Some(presentation);
    Ok(())
}

fn write_clipboard(
    input: &mut impl std::io::Read,
    output: &SharedOutput,
    request_id: &str,
    generation: u64,
    content: ClipboardContent
) -> Result<Option<u64>, nanika_protocol::FrameError> {
    let service_request_id = format!("host-{request_id}");
    send_frame(
        output,
        &Message::HostRequest {
            request_id: service_request_id.clone(),
            parent_request_id: request_id.to_owned(),
            generation,
            request: HostServiceRequest::WriteClipboard { content }
        }
    )?;
    loop {
        match read_frame(input)? {
            Some(Message::HostResponse {
                request_id: response_id,
                parent_request_id,
                generation: response_generation,
                response
            }) if response_id == service_request_id
                && parent_request_id == request_id
                && response_generation == generation =>
            {
                match response {
                    HostServiceResponse::ClipboardWritten { revision } => {
                        return Ok(Some(revision));
                    }
                    HostServiceResponse::Failed { failure } => {
                        send_frame(
                            output,
                            &Message::Error {
                                request_id: Some(request_id.to_owned()),
                                error: failure.into_error("error-host-service")
                            }
                        )?;
                        return Ok(None);
                    }
                    _ => continue
                }
            }
            Some(Message::Error { .. }) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unexpected host error outside a service response."
                )
                .into());
            }
            Some(_) => {}
            None => return Ok(None)
        }
    }
}

fn _send_error(
    output: &SharedOutput,
    request_id: Option<String>,
    code: &str,
    reference: &str,
    detail: &str
) -> Result<(), nanika_protocol::FrameError> {
    let error = nanika_protocol::ProtocolError::new(code, nanika_protocol::message(reference, []))
        .with_detail(detail);
    send_frame(output, &Message::Error { request_id, error })
}

fn send_frame(output: &SharedOutput, message: &Message) -> Result<(), nanika_protocol::FrameError> {
    write_frame(
        &mut *output.lock().unwrap_or_else(|error| error.into_inner()),
        message
    )
}

fn request_id(message: &Message) -> Option<String> {
    match message {
        Message::Initialize { request_id, .. }
        | Message::Initialized { request_id, .. }
        | Message::CatalogRead { request_id }
        | Message::CatalogBatch { request_id, .. }
        | Message::Query { request_id, .. }
        | Message::Snapshot { request_id, .. }
        | Message::Invoke { request_id, .. }
        | Message::Result { request_id, .. }
        | Message::ViewEvent { request_id, .. }
        | Message::ViewUpdated { request_id, .. }
        | Message::ViewClose { request_id, .. }
        | Message::ViewClosed { request_id, .. }
        | Message::TextRead { request_id, .. }
        | Message::TextReadResult { request_id, .. }
        | Message::Cancel { request_id, .. }
        | Message::Refresh { request_id, .. }
        | Message::Refreshed { request_id, .. }
        | Message::ConfigurationChanged { request_id, .. }
        | Message::ConfigurationProgress { request_id, .. }
        | Message::ConfigurationApplied { request_id }
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
        Message::CatalogApplied { .. }
        | Message::CandidatesChanged
        | Message::ViewsChanged { .. }
        | Message::PrepareEntries { .. } => None
    }
}

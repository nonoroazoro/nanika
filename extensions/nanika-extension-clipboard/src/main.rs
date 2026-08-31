//! Clipboard history extension process entry point.

use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use nanika_extension_clipboard::{
    COPY_ACTION_ID, ClipboardEntry, ClipboardMonitor, ClipboardViewState, ClipboardWorker,
    OPEN_COMMAND_ID, RuntimePaths, clipboard_view,
};
use nanika_protocol::{
    ClipboardContent, HostServiceRequest, HostServiceResponse, Message, NavigationEffect,
    PROTOCOL_NAME, SettingsContribution, ViewEvent, read_frame, write_frame,
};

const VIEW_ID: &str = "clipboard.history";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = RuntimePaths::parse(std::env::args().skip(1))?;
    let entries = Arc::new(RwLock::new(Vec::<ClipboardEntry>::new()));
    let worker = ClipboardWorker::spawn(
        paths.database_path(),
        paths.payload_root(),
        Arc::clone(&entries),
    )?;
    let monitor = ClipboardMonitor::spawn(&worker)?;
    worker.capture_background();
    let mut input = BufReader::new(stdin().lock());
    let mut output = BufWriter::new(stdout().lock());
    let mut initialized = false;
    let mut view_state = None;
    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Initialize {
                request_id,
                protocol,
            } if protocol == PROTOCOL_NAME => {
                initialized = true;
                write_frame(
                    &mut output,
                    &Message::Initialized {
                        request_id,
                        protocol: PROTOCOL_NAME.to_owned(),
                    },
                )?;
            }
            Message::Initialize { request_id, .. } => write_error(
                &mut output,
                Some(request_id),
                "unsupported_protocol",
                "the requested extension protocol is unsupported",
            )?,
            message if !initialized => write_error(
                &mut output,
                request_id(&message),
                "not_initialized",
                "initialize must complete before other requests",
            )?,
            Message::Query {
                request_id,
                generation,
                ..
            } => match worker.last_error() {
                Some(message) => write_error(
                    &mut output,
                    Some(request_id),
                    "clipboard_worker_failed",
                    &message,
                )?,
                None => write_frame(
                    &mut output,
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
                if entry_id != OPEN_COMMAND_ID || action_id != "command.execute" {
                    write_error(
                        &mut output,
                        Some(request_id),
                        "unknown_action",
                        "clipboard command or action does not exist",
                    )?;
                    continue;
                }
                let mut state = ClipboardViewState::new();
                let view = clipboard_view(
                    &mut state,
                    &entries.read().unwrap_or_else(|error| error.into_inner()),
                );
                view_state = Some(state);
                write_frame(
                    &mut output,
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
            }
            Message::ViewEvent {
                request_id,
                generation,
                view_id,
                revision,
                event,
            } if view_id == VIEW_ID => handle_view_event(
                &mut input,
                &mut output,
                &worker,
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
                write_frame(
                    &mut output,
                    &Message::ViewClosed {
                        request_id,
                        view_id,
                    },
                )?;
            }
            Message::Refresh {
                request_id,
                generation,
            } => match capture_now(&worker) {
                Ok(()) => write_frame(
                    &mut output,
                    &Message::Refreshed {
                        request_id,
                        generation,
                    },
                )?,
                Err(message) => {
                    write_error(&mut output, Some(request_id), "refresh_failed", &message)?
                }
            },
            Message::Cancel { .. } => {}
            Message::GetSettings { request_id } => write_frame(
                &mut output,
                &Message::Settings {
                    request_id,
                    contribution: empty_settings("Clipboard history"),
                },
            )?,
            Message::UpdateSettings {
                request_id,
                updates,
            } if updates.is_empty() => write_frame(
                &mut output,
                &Message::SettingsUpdated {
                    request_id,
                    contribution: empty_settings("Clipboard history"),
                },
            )?,
            Message::Shutdown { request_id } => {
                write_frame(&mut output, &Message::ShutdownAck { request_id })?;
                break;
            }
            message => write_error(
                &mut output,
                request_id(&message),
                "unsupported_message",
                "the clipboard extension received an unsupported message",
            )?,
        }
    }
    monitor.shutdown();
    worker.shutdown();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_view_event(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    worker: &ClipboardWorker,
    entries: &Arc<RwLock<Vec<ClipboardEntry>>>,
    view_state: &mut Option<ClipboardViewState>,
    request_id: String,
    generation: u64,
    revision: u64,
    event: ViewEvent,
) -> Result<(), nanika_protocol::FrameError> {
    let Some(state) = view_state.as_mut() else {
        return write_error(
            output,
            Some(request_id),
            "unknown_view",
            "clipboard view is not open",
        );
    };
    if state.revision != revision {
        return write_error(
            output,
            Some(request_id),
            "stale_view",
            "clipboard view revision is stale",
        );
    }
    match event {
        ViewEvent::SearchChanged { text } => {
            state.query = text.chars().take(4_096).collect();
            state.visible_limit = 100;
        }
        ViewEvent::SelectionChanged { item_id } => state.selected_item_id = item_id,
        ViewEvent::FilterChanged { filter_id, value }
            if filter_id == "contentType"
                && matches!(value.as_str(), "all" | "text" | "files" | "images") =>
        {
            state.content_type = value;
            state.visible_limit = 100;
        }
        ViewEvent::LoadMore { cursor } if cursor == state.visible_limit.to_string() => {
            state.visible_limit = state.visible_limit.saturating_add(100).min(500);
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
                return write_error(
                    output,
                    Some(request_id),
                    "unknown_action",
                    "clipboard entry or action does not exist",
                );
            };
            if write_clipboard(input, output, &request_id, generation, content)? {
                if let Some(item_id) = item_id {
                    worker.mark_used(item_id);
                }
                write_frame(
                    output,
                    &Message::ViewUpdated {
                        request_id,
                        generation,
                        view_id: VIEW_ID.to_owned(),
                        revision: state.revision,
                        effect: NavigationEffect::Close,
                        view: None,
                    },
                )?;
            }
            return Ok(());
        }
        _ => {
            return write_error(
                output,
                Some(request_id),
                "invalid_view_event",
                "clipboard view event is invalid",
            );
        }
    }
    state.revision = state.revision.saturating_add(1);
    let view = clipboard_view(
        state,
        &entries.read().unwrap_or_else(|error| error.into_inner()),
    );
    write_frame(
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

fn capture_now(worker: &ClipboardWorker) -> Result<(), String> {
    worker
        .capture()?
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "clipboard capture did not finish before the deadline".to_owned())?
}

fn write_clipboard(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    request_id: &str,
    generation: u64,
    content: ClipboardContent,
) -> Result<bool, nanika_protocol::FrameError> {
    let service_request_id = format!("host-{request_id}");
    write_frame(
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
                response: HostServiceResponse::ClipboardWritten,
            }) if response_id == service_request_id
                && parent_request_id == request_id
                && response_generation == generation =>
            {
                return Ok(true);
            }
            Some(Message::Error {
                request_id: Some(response_id),
                code,
                message,
            }) if response_id == service_request_id => {
                write_error(output, Some(request_id.to_owned()), &code, &message)?;
                return Ok(false);
            }
            Some(_) => {}
            None => return Ok(false),
        }
    }
}

fn write_error(
    output: &mut impl std::io::Write,
    request_id: Option<String>,
    code: &str,
    message: &str,
) -> Result<(), nanika_protocol::FrameError> {
    write_frame(
        output,
        &Message::Error {
            request_id,
            code: code.to_owned(),
            message: message.to_owned(),
        },
    )
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
        | Message::GetSettings { request_id }
        | Message::Settings { request_id, .. }
        | Message::UpdateSettings { request_id, .. }
        | Message::SettingsUpdated { request_id, .. }
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. }
        | Message::Shutdown { request_id }
        | Message::ShutdownAck { request_id } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
    }
}

fn empty_settings(title: &str) -> SettingsContribution {
    SettingsContribution {
        title: title.to_owned(),
        fields: Vec::new(),
    }
}

//! Clipboard history extension process entry point.

use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use nanika_extension_clipboard::{
    ClipboardEntry, ClipboardMonitor, ClipboardWorker, RESTORE_ACTION_ID, RuntimePaths,
};
use nanika_protocol::{
    ClipboardContent, HostServiceRequest, HostServiceResponse, Message, PROTOCOL_NAME,
    SettingsContribution, read_frame, write_frame,
};

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
                        entries: entries
                            .read()
                            .unwrap_or_else(|error| error.into_inner())
                            .iter()
                            .take(5_000)
                            .map(ClipboardEntry::candidate)
                            .collect(),
                    },
                )?,
            },
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id,
            } => {
                let content = entries
                    .read()
                    .unwrap_or_else(|error| error.into_inner())
                    .iter()
                    .find(|entry| entry.entry_id == entry_id)
                    .filter(|_| action_id == RESTORE_ACTION_ID)
                    .map(|entry| entry.content.clone());
                match content {
                    Some(content) => {
                        if invoke_host(&mut input, &mut output, request_id, generation, content)? {
                            worker.mark_used(entry_id);
                        }
                    }
                    None => write_error(
                        &mut output,
                        Some(request_id),
                        "unknown_action",
                        "clipboard entry or action does not exist",
                    )?,
                }
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

fn capture_now(worker: &ClipboardWorker) -> Result<(), String> {
    worker
        .capture()?
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "clipboard capture did not finish before the deadline".to_owned())?
}

fn invoke_host(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    request_id: String,
    generation: u64,
    content: ClipboardContent,
) -> Result<bool, nanika_protocol::FrameError> {
    let service_request_id = format!("host-{request_id}");
    write_frame(
        output,
        &Message::HostRequest {
            request_id: service_request_id.clone(),
            parent_request_id: request_id.clone(),
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
                write_frame(
                    output,
                    &Message::Result {
                        request_id,
                        generation,
                    },
                )?;
                return Ok(true);
            }
            Some(Message::Error {
                request_id: Some(response_id),
                code,
                message,
            }) if response_id == service_request_id => {
                write_error(output, Some(request_id), &code, &message)?;
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

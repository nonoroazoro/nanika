//! Application extension process entry point.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};

use nanika_config::ConfigStore;
use nanika_extension_application::{
    ApplicationConfig, ApplicationEntry, DiscoveryWorker, RuntimeEvent, RuntimePaths,
    select_candidates,
};
use nanika_protocol::{
    HostServiceRequest, HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame,
};

#[path = "PendingInvocation.rs"]
mod pending_invocation;

use pending_invocation::PendingInvocation;

const EVENT_CAPACITY: usize = 8;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = RuntimePaths::resolve(std::env::args().skip(1))?;
    let config_store = ConfigStore::open(&paths.data_root, &paths.config_root)?;
    let mut config = ApplicationConfig::load(&config_store)?;
    let database_path = paths.database_path();
    let icon_root = paths.icon_root();
    let entries = Arc::new(RwLock::new(Vec::<ApplicationEntry>::new()));
    let (event_sender, events) = mpsc::sync_channel(EVENT_CAPACITY);
    let _reader = spawn_protocol_reader(event_sender.clone())?;
    let worker = DiscoveryWorker::spawn(
        database_path,
        icon_root,
        config_store.clone(),
        Arc::clone(&entries),
        event_sender,
    )?;
    let mut output = BufWriter::new(stdout().lock());
    let mut initialized = false;
    let mut refresh_requests = HashMap::<String, u64>::new();
    let mut pending_invocations = HashMap::<String, PendingInvocation>::new();
    let mut latest_generation = 1_u64;
    while let Ok(event) = events.recv() {
        match event {
            RuntimeEvent::Protocol(message) => match message {
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
                    query,
                } => {
                    latest_generation = latest_generation.max(generation);
                    write_snapshot(&mut output, &entries, &request_id, generation, &query, true)?;
                }
                Message::Refresh {
                    request_id,
                    generation,
                } => {
                    latest_generation = latest_generation.max(generation);
                    if let Err(message) = worker.refresh(Some(request_id.clone()), generation) {
                        write_error(&mut output, Some(request_id), "refresh_failed", &message)?;
                    } else {
                        refresh_requests.insert(request_id, generation);
                    }
                }
                Message::Cancel {
                    request_id,
                    generation,
                } => {
                    if refresh_requests.get(&request_id) == Some(&generation) {
                        worker.cancel(generation);
                    }
                }
                Message::Invoke {
                    request_id,
                    generation,
                    entry_id,
                    action_id,
                } => {
                    latest_generation = latest_generation.max(generation);
                    let descriptor = entries
                        .read()
                        .unwrap_or_else(|error| error.into_inner())
                        .iter()
                        .find(|entry| entry.entry_id == entry_id)
                        .filter(|_| action_id == nanika_extension_application::RUN_ACTION_ID)
                        .ok_or_else(|| "application entry or action does not exist".to_owned())
                        .and_then(|entry| {
                            entry.launch_descriptor().map_err(|error| error.to_string())
                        });
                    match descriptor {
                        Ok(descriptor) => {
                            let service_request_id = format!("host-{request_id}");
                            write_frame(
                                &mut output,
                                &Message::HostRequest {
                                    request_id: service_request_id.clone(),
                                    parent_request_id: request_id.clone(),
                                    generation,
                                    request: HostServiceRequest::Launch { descriptor },
                                },
                            )?;
                            pending_invocations.insert(
                                service_request_id,
                                PendingInvocation {
                                    request_id,
                                    generation,
                                },
                            );
                        }
                        Err(message) => {
                            write_error(&mut output, Some(request_id), "unknown_action", &message)?
                        }
                    }
                }
                Message::GetSettings { request_id } => {
                    let contribution = config.settings();
                    contribution.validate().map_err(std::io::Error::other)?;
                    write_frame(
                        &mut output,
                        &Message::Settings {
                            request_id,
                            contribution,
                        },
                    )?;
                }
                Message::UpdateSettings {
                    request_id,
                    updates,
                } => match config.update(&config_store, updates) {
                    Ok(updated) => {
                        config = updated;
                        latest_generation = latest_generation.saturating_add(1);
                        match worker.refresh(None, latest_generation) {
                            Ok(()) => write_frame(
                                &mut output,
                                &Message::SettingsUpdated {
                                    request_id,
                                    contribution: config.settings(),
                                },
                            )?,
                            Err(message) => write_error(
                                &mut output,
                                Some(request_id),
                                "settings_refresh_failed",
                                &message,
                            )?,
                        }
                    }
                    Err(error) => write_error(
                        &mut output,
                        Some(request_id),
                        "invalid_settings",
                        &error.to_string(),
                    )?,
                },
                Message::HostResponse {
                    request_id,
                    parent_request_id,
                    generation: response_generation,
                    response: HostServiceResponse::Launched,
                } => {
                    if let Some(pending) = pending_invocations.remove(&request_id) {
                        if parent_request_id == pending.request_id
                            && response_generation == pending.generation
                        {
                            write_frame(
                                &mut output,
                                &Message::Result {
                                    request_id: pending.request_id,
                                    generation: pending.generation,
                                    effect: nanika_protocol::NavigationEffect::Close,
                                },
                            )?;
                        } else {
                            write_error(
                                &mut output,
                                Some(pending.request_id),
                                "invalid_host_response",
                                "host response does not match the pending application invocation",
                            )?;
                        }
                    }
                }
                Message::Error {
                    request_id: Some(service_request_id),
                    code,
                    message,
                } if pending_invocations.contains_key(&service_request_id) => {
                    if let Some(pending) = pending_invocations.remove(&service_request_id) {
                        write_error(&mut output, Some(pending.request_id), &code, &message)?;
                    }
                }
                Message::Shutdown { request_id } => {
                    write_frame(&mut output, &Message::ShutdownAck { request_id })?;
                    break;
                }
                message => write_error(
                    &mut output,
                    request_id(&message),
                    "unsupported_message",
                    "the application extension received an unsupported message",
                )?,
            },
            RuntimeEvent::CandidatesChanged => {
                if initialized {
                    write_frame(&mut output, &Message::CandidatesChanged)?;
                }
            }
            RuntimeEvent::ProtocolClosed => break,
            RuntimeEvent::ProtocolError(message) => {
                return Err(std::io::Error::other(format!(
                    "application protocol input failed: {message}"
                ))
                .into());
            }
            RuntimeEvent::ScanFinished {
                request_id: Some(request_id),
                response_generation,
                result: Ok(report),
            } if !report.cancelled => {
                refresh_requests.remove(&request_id);
                if initialized {
                    write_frame(&mut output, &Message::CandidatesChanged)?;
                }
                if report.complete && report.warnings == 0 {
                    write_frame(
                        &mut output,
                        &Message::Refreshed {
                            request_id,
                            generation: response_generation,
                        },
                    )?;
                } else {
                    write_error(
                        &mut output,
                        Some(request_id),
                        "refresh_incomplete",
                        &format!(
                            "application scan was incomplete with {} path errors",
                            report.warnings
                        ),
                    )?;
                }
            }
            RuntimeEvent::ScanFinished {
                request_id,
                response_generation: _,
                result: Err(message),
            } => {
                if initialized {
                    write_frame(&mut output, &Message::CandidatesChanged)?;
                }
                if let Some(request_id) = request_id {
                    refresh_requests.remove(&request_id);
                    write_error(&mut output, Some(request_id), "refresh_failed", &message)?;
                }
                return Err(std::io::Error::other(format!(
                    "application discovery failed: {message}"
                ))
                .into());
            }
            RuntimeEvent::ScanFinished {
                request_id,
                result: Ok(report),
                ..
            } => {
                if let Some(request_id) = request_id {
                    refresh_requests.remove(&request_id);
                }
                if initialized {
                    write_frame(&mut output, &Message::CandidatesChanged)?;
                }
                if !report.cancelled && (!report.complete || report.warnings > 0) {
                    eprintln!(
                        "application scan was incomplete with {} path errors",
                        report.warnings
                    );
                }
            }
        }
    }
    worker.shutdown();
    Ok(())
}

fn write_snapshot(
    output: &mut impl std::io::Write,
    entries: &RwLock<Vec<ApplicationEntry>>,
    request_id: &str,
    generation: u64,
    query: &str,
    complete: bool,
) -> Result<(), nanika_protocol::FrameError> {
    let entries = entries.read().unwrap_or_else(|error| error.into_inner());
    let candidates = select_candidates(&entries, query);
    write_frame(
        output,
        &Message::Snapshot {
            request_id: request_id.to_owned(),
            generation,
            complete,
            entries: candidates,
        },
    )
}

fn spawn_protocol_reader(
    events: SyncSender<RuntimeEvent>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("nanika-application-protocol".to_owned())
        .spawn(move || {
            let mut input = BufReader::new(stdin().lock());
            loop {
                match read_frame(&mut input) {
                    Ok(Some(message)) => {
                        if events.send(RuntimeEvent::Protocol(message)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = events.send(RuntimeEvent::ProtocolClosed);
                        break;
                    }
                    Err(error) => {
                        let _ = events.send(RuntimeEvent::ProtocolError(error.to_string()));
                        break;
                    }
                }
            }
        })
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
        Message::Initialized { request_id, .. } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
        Message::CandidatesChanged => None,
    }
}

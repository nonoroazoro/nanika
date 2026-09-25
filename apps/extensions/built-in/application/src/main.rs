use std::collections::HashMap;
use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};

use nanika_extension_application::{
    ApplicationConfig, ApplicationEntry, DiscoveryWorker, RuntimeEvent, RuntimePaths,
    select_candidates,
};
use nanika_protocol::{HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame};

#[path = "PendingInvocation.rs"]
mod pending_invocation;

use pending_invocation::PendingInvocation;

const EVENT_CAPACITY: usize = 8;

struct PendingConfiguration {
    generation: u64,
    previous: ApplicationConfig,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = RuntimePaths::resolve(std::env::args().skip(1))?;
    let mut output = BufWriter::new(stdout().lock());
    let (initialize_request_id, initial_configuration) = {
        let mut input = BufReader::new(stdin().lock());
        match read_frame(&mut input)? {
            Some(Message::Initialize {
                request_id,
                protocol,
                configuration,
            }) if protocol == PROTOCOL_NAME => (request_id, configuration),
            Some(Message::Initialize { request_id, .. }) => {
                write_error(
                    &mut output,
                    Some(request_id),
                    "unsupported_protocol",
                    "the requested extension protocol is unsupported",
                )?;
                return Ok(());
            }
            Some(message) => {
                write_error(
                    &mut output,
                    request_id(&message),
                    "not_initialized",
                    "initialize must be the first request",
                )?;
                return Ok(());
            }
            None => return Ok(()),
        }
    };
    let initial_config = match ApplicationConfig::from_configuration(&initial_configuration) {
        Ok(config) => config,
        Err(error) => {
            write_error(
                &mut output,
                Some(initialize_request_id),
                "invalid_configuration",
                &error.to_string(),
            )?;
            return Ok(());
        }
    };
    let config = Arc::new(RwLock::new(initial_config));
    let database_path = paths.database_path();
    let icon_root = paths.icon_root();
    let entries = Arc::new(RwLock::new(Vec::<ApplicationEntry>::new()));
    let (event_sender, events) = mpsc::sync_channel(EVENT_CAPACITY);
    let worker = DiscoveryWorker::spawn(
        database_path,
        icon_root,
        Arc::clone(&config),
        Arc::clone(&entries),
        event_sender.clone(),
    )?;
    write_frame(
        &mut output,
        &Message::Initialized {
            request_id: initialize_request_id,
            protocol: PROTOCOL_NAME.to_owned(),
        },
    )?;
    let _reader = spawn_protocol_reader(event_sender)?;
    let mut refresh_requests = HashMap::<String, u64>::new();
    let mut configuration_requests = HashMap::<String, PendingConfiguration>::new();
    let mut pending_invocations = HashMap::<String, PendingInvocation>::new();
    let mut latest_generation = 1_u64;
    while let Ok(event) = events.recv() {
        match event {
            RuntimeEvent::Protocol(message) => match message {
                Message::Query {
                    request_id,
                    generation,
                    query,
                } => {
                    latest_generation = latest_generation.max(generation);
                    write_snapshot(&mut output, &entries, &request_id, generation, &query, true)?;
                }
                Message::PrepareEntries {
                    generation,
                    entry_ids,
                } => worker.prepare_entries(generation, entry_ids),
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
                        .ok_or_else(|| "application entry or action does not exist".to_owned())
                        .and_then(|entry| {
                            entry
                                .host_request(&action_id)
                                .map_err(|error| error.to_string())
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
                                    request: descriptor,
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
                Message::ConfigurationChanged {
                    request_id,
                    configuration,
                } => match ApplicationConfig::from_configuration(&configuration) {
                    Ok(updated) => {
                        let previous = {
                            let mut current =
                                config.write().unwrap_or_else(|error| error.into_inner());
                            std::mem::replace(&mut *current, updated)
                        };
                        latest_generation = latest_generation.saturating_add(1);
                        match worker.refresh(Some(request_id.clone()), latest_generation) {
                            Ok(()) => {
                                configuration_requests.insert(
                                    request_id,
                                    PendingConfiguration {
                                        generation: latest_generation,
                                        previous,
                                    },
                                );
                            }
                            Err(message) => {
                                *config.write().unwrap_or_else(|error| error.into_inner()) =
                                    previous;
                                write_error(
                                    &mut output,
                                    Some(request_id),
                                    "configuration_apply_failed",
                                    &message,
                                )?;
                            }
                        }
                    }
                    Err(error) => write_error(
                        &mut output,
                        Some(request_id),
                        "invalid_configuration",
                        &error.to_string(),
                    )?,
                },
                Message::HostResponse {
                    request_id,
                    parent_request_id,
                    generation: response_generation,
                    response: HostServiceResponse::Launched | HostServiceResponse::PathRevealed,
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
                                    effect: nanika_protocol::NavigationEffect::Dismiss,
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
            RuntimeEvent::ScanProgress {
                request_id,
                progress,
            } => {
                if configuration_requests.contains_key(&request_id) {
                    write_frame(
                        &mut output,
                        &Message::ConfigurationProgress {
                            request_id,
                            progress,
                        },
                    )?;
                }
            }
            RuntimeEvent::CandidatesChanged => {
                write_frame(&mut output, &Message::CandidatesChanged)?;
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
            } if configuration_requests.contains_key(&request_id) => {
                let pending = configuration_requests
                    .remove(&request_id)
                    .expect("guarded configuration request must exist");
                write_frame(&mut output, &Message::CandidatesChanged)?;
                if pending.generation != response_generation {
                    *config.write().unwrap_or_else(|error| error.into_inner()) = pending.previous;
                    write_error(
                        &mut output,
                        Some(request_id),
                        "configuration_apply_failed",
                        "application scan returned the wrong configuration generation",
                    )?;
                } else if report.cancelled {
                    *config.write().unwrap_or_else(|error| error.into_inner()) = pending.previous;
                    write_error(
                        &mut output,
                        Some(request_id),
                        "configuration_apply_failed",
                        "application scan was cancelled before configuration was applied",
                    )?;
                } else {
                    if !report.complete || report.warnings > 0 {
                        eprintln!(
                            "application configuration scan was incomplete with {} path errors",
                            report.warnings
                        );
                    }
                    write_frame(&mut output, &Message::ConfigurationApplied { request_id })?;
                }
            }
            RuntimeEvent::ScanFinished {
                request_id: Some(request_id),
                response_generation,
                result: Ok(report),
            } if !report.cancelled => {
                refresh_requests.remove(&request_id);
                write_frame(&mut output, &Message::CandidatesChanged)?;
                if !report.complete || report.warnings > 0 {
                    eprintln!(
                        "application scan was incomplete with {} path errors",
                        report.warnings
                    );
                }
                // Refresh acknowledges completion; path failures stay in the scan log
                // and partial scan state instead of becoming user-facing errors.
                write_frame(
                    &mut output,
                    &Message::Refreshed {
                        request_id,
                        generation: response_generation,
                    },
                )?;
            }
            RuntimeEvent::ScanFinished {
                request_id,
                response_generation: _,
                result: Err(message),
            } => {
                write_frame(&mut output, &Message::CandidatesChanged)?;
                if let Some(request_id) = request_id {
                    if let Some(pending) = configuration_requests.remove(&request_id) {
                        *config.write().unwrap_or_else(|error| error.into_inner()) =
                            pending.previous;
                        write_error(
                            &mut output,
                            Some(request_id),
                            "configuration_apply_failed",
                            &message,
                        )?;
                    } else {
                        refresh_requests.remove(&request_id);
                        write_error(&mut output, Some(request_id), "refresh_failed", &message)?;
                    }
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
                write_frame(&mut output, &Message::CandidatesChanged)?;
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
        | Message::ConfigurationChanged { request_id, .. }
        | Message::ConfigurationProgress { request_id, .. }
        | Message::ConfigurationApplied { request_id }
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. }
        | Message::Shutdown { request_id }
        | Message::ShutdownAck { request_id } => Some(request_id.clone()),
        Message::Initialized { request_id, .. } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
        Message::CandidatesChanged
        | Message::ViewInvalidated { .. }
        | Message::PrepareEntries { .. } => None,
    }
}

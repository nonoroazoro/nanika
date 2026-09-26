use std::collections::HashMap;
use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};

use nanika_extension_application::{
    ApplicationConfig, ApplicationEntry, DiscoveryWorker, RuntimeEvent, RuntimePaths,
};
use nanika_protocol::{HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame};

#[path = "DiscoveryRuntime.rs"]
mod discovery_runtime;

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
    let entries = Arc::new(RwLock::new(HashMap::<String, ApplicationEntry>::new()));
    let (event_sender, events) = mpsc::sync_channel(EVENT_CAPACITY);
    let worker = DiscoveryWorker::spawn(
        database_path,
        icon_root,
        Arc::clone(&config),
        Arc::clone(&entries),
        event_sender.clone(),
    )?;
    let discovery = discovery_runtime::DiscoveryRuntime::new(events, worker);
    write_frame(
        &mut output,
        &Message::Initialized {
            request_id: initialize_request_id,
            protocol: PROTOCOL_NAME.to_owned(),
        },
    )?;
    let _reader = spawn_protocol_reader(event_sender)?;
    let mut startup_pending = true;
    let mut catalog = nanika_protocol::CatalogPublisher::default();
    let mut refresh_requests = HashMap::<String, (u64, u64)>::new();
    let mut configuration_requests = HashMap::<String, PendingConfiguration>::new();
    let mut pending_invocations = HashMap::<String, PendingInvocation>::new();
    let mut latest_generation = 1_u64;
    while let Ok(event) = discovery.receive() {
        match event {
            RuntimeEvent::Protocol(message) => match message {
                Message::CatalogRead { request_id } => match catalog.read() {
                    Ok(batch) => {
                        write_frame(&mut output, &Message::CatalogBatch { request_id, batch })?
                    }
                    Err(error) => {
                        write_error(&mut output, Some(request_id), "catalog_failed", &error)?
                    }
                },
                Message::CatalogApplied { transaction } => {
                    if catalog
                        .acknowledge(transaction)
                        .map_err(std::io::Error::other)?
                    {
                        write_frame(&mut output, &Message::CandidatesChanged)?;
                    }
                }
                Message::PrepareEntries {
                    generation,
                    entry_ids,
                } => discovery.worker.prepare_entries(generation, entry_ids),
                Message::Refresh {
                    request_id,
                    generation,
                } => {
                    latest_generation = latest_generation.max(generation);
                    if startup_pending {
                        refresh_requests.insert(request_id, (generation, 1));
                        continue;
                    }
                    latest_generation = latest_generation.saturating_add(1);
                    if let Err(message) = discovery
                        .worker
                        .refresh(Some(request_id.clone()), latest_generation)
                    {
                        write_error(&mut output, Some(request_id), "refresh_failed", &message)?;
                    } else {
                        refresh_requests.insert(request_id, (generation, latest_generation));
                    }
                }
                Message::Cancel {
                    request_id,
                    generation,
                } => {
                    if let Some((expected, scan)) = refresh_requests.get(&request_id)
                        && *expected == generation
                    {
                        discovery.worker.cancel(*scan);
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
                        .get(&entry_id)
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
                        match discovery
                            .worker
                            .refresh(Some(request_id.clone()), latest_generation)
                        {
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
            RuntimeEvent::CatalogUpdated { entry_ids } => {
                let current = entries.read().unwrap_or_else(|error| error.into_inner());
                let updated = entry_ids
                    .iter()
                    .filter_map(|id| current.get(id))
                    .map(|entry| entry.candidate());
                let removed = entry_ids
                    .iter()
                    .filter(|id| !current.contains_key(*id))
                    .cloned();
                let changed = catalog.update(updated, removed);
                drop(current);
                if changed {
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
            } if configuration_requests.contains_key(&request_id) => {
                let pending = configuration_requests
                    .remove(&request_id)
                    .expect("guarded configuration request must exist");
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
                let Some((generation, expected_scan)) = refresh_requests.remove(&request_id) else {
                    continue;
                };
                if response_generation != expected_scan {
                    write_error(
                        &mut output,
                        Some(request_id),
                        "refresh_failed",
                        "application scan returned the wrong generation",
                    )?;
                    continue;
                }
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
                        generation,
                    },
                )?;
            }
            RuntimeEvent::ScanFinished {
                request_id,
                response_generation: _,
                result: Err(message),
            } => {
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
                    write_error(
                        &mut output,
                        Some(request_id),
                        "refresh_cancelled",
                        "application scan was cancelled",
                    )?;
                } else {
                    startup_pending = false;
                    // Launcher opens during startup share its scan and receive their own acknowledgement.
                    for (request_id, (generation, _)) in refresh_requests.drain() {
                        if report.cancelled {
                            write_error(
                                &mut output,
                                Some(request_id),
                                "refresh_cancelled",
                                "application scan was cancelled",
                            )?;
                        } else {
                            write_frame(
                                &mut output,
                                &Message::Refreshed {
                                    request_id,
                                    generation,
                                },
                            )?;
                        }
                    }
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
    discovery.shutdown().map_err(std::io::Error::other)?;
    Ok(())
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
        | Message::Cancel { request_id, .. }
        | Message::Refresh { request_id, .. }
        | Message::Refreshed { request_id, .. }
        | Message::ConfigurationChanged { request_id, .. }
        | Message::ConfigurationProgress { request_id, .. }
        | Message::ConfigurationApplied { request_id }
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. } => Some(request_id.clone()),
        Message::Initialized { request_id, .. } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
        Message::CatalogApplied { .. }
        | Message::CandidatesChanged
        | Message::ViewInvalidated { .. }
        | Message::PrepareEntries { .. } => None,
    }
}

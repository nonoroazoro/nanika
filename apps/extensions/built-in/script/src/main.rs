use nanika_extension_script::{RUN_ACTION_ID, ScriptConfig, ScriptEntry};
use nanika_protocol::{
    HostServiceRequest, HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame,
};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::{Arc, RwLock, mpsc};

#[path = "RuntimeEvent.rs"]
mod runtime_event;
#[path = "ScriptDiscovery.rs"]
mod script_discovery;
#[path = "ScriptScan.rs"]
mod script_scan;
use runtime_event::RuntimeEvent;
use script_discovery::ScriptDiscovery;
use script_scan::ScriptScan;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = BufReader::new(stdin().lock());
    let mut output = BufWriter::new(stdout().lock());
    let (initialize_request_id, configuration) = match read_frame(&mut input)? {
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
    };
    let mut config = match ScriptConfig::from_configuration(&configuration) {
        Ok(config) => config,
        Err(message) => {
            write_error(
                &mut output,
                Some(initialize_request_id),
                "invalid_configuration",
                &message,
            )?;
            return Ok(());
        }
    };
    // The initialization reader must release stdin before the protocol thread takes ownership.
    drop(input);
    let scripts = Arc::new(RwLock::new(BTreeMap::<String, ScriptEntry>::new()));
    let (sender, events) = mpsc::sync_channel(8);
    let discovery = ScriptDiscovery::spawn(events, sender.clone(), Arc::clone(&scripts))?;
    let mut scan_generation = 1;
    discovery.scan(ScriptScan {
        request_id: None,
        generation: scan_generation,
        config: config.clone(),
        configuration: false,
    })?;
    write_frame(
        &mut output,
        &Message::Initialized {
            request_id: initialize_request_id,
            protocol: PROTOCOL_NAME.to_owned(),
        },
    )?;
    let _reader = std::thread::Builder::new()
        .name("nanika-script-protocol".to_owned())
        .spawn(move || {
            let mut input = BufReader::new(stdin().lock());
            loop {
                let event = match read_frame(&mut input) {
                    Ok(Some(message)) => RuntimeEvent::Protocol(message),
                    Ok(None) => RuntimeEvent::ProtocolClosed,
                    Err(error) => RuntimeEvent::ProtocolError(error.to_string()),
                };
                let finished = !matches!(event, RuntimeEvent::Protocol(_));
                if sender.send(event).is_err() || finished {
                    break;
                }
            }
        })?;
    let mut startup_pending = true;
    let mut catalog = nanika_protocol::CatalogPublisher::default();
    let mut refreshes = HashMap::<String, (u64, u64)>::new();
    let mut configurations = HashMap::<String, ScriptConfig>::new();
    let mut invocations = HashMap::<String, (String, u64)>::new();
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
                Message::Invoke {
                    request_id,
                    generation,
                    entry_id,
                    action_id,
                } => {
                    let descriptor = scripts
                        .read()
                        .unwrap_or_else(|error| error.into_inner())
                        .get(&entry_id)
                        .filter(|_| action_id == RUN_ACTION_ID)
                        .ok_or_else(|| "script entry or action does not exist".to_owned())
                        .and_then(|script| script.launch_descriptor());
                    match descriptor {
                        Ok(descriptor) => {
                            let service_id = format!("host-{request_id}");
                            write_frame(
                                &mut output,
                                &Message::HostRequest {
                                    request_id: service_id.clone(),
                                    parent_request_id: request_id.clone(),
                                    generation,
                                    request: HostServiceRequest::Launch { descriptor },
                                },
                            )?;
                            invocations.insert(service_id, (request_id, generation));
                        }
                        Err(message) => write_error(
                            &mut output,
                            Some(request_id),
                            "script_unavailable",
                            &message,
                        )?,
                    }
                }
                Message::HostResponse {
                    request_id,
                    parent_request_id,
                    generation,
                    response: HostServiceResponse::Launched,
                } => {
                    if let Some((id, expected)) = invocations.remove(&request_id) {
                        if id == parent_request_id && expected == generation {
                            write_frame(
                                &mut output,
                                &Message::Result {
                                    request_id: id,
                                    generation,
                                    effect: nanika_protocol::NavigationEffect::Dismiss,
                                },
                            )?;
                        } else {
                            write_error(
                                &mut output,
                                Some(id),
                                "invalid_host_response",
                                "host response does not match the pending script invocation",
                            )?;
                        }
                    }
                }
                Message::Error {
                    request_id: Some(id),
                    code,
                    message,
                } if invocations.contains_key(&id) => {
                    let (request_id, _) = invocations.remove(&id).expect("pending invocation");
                    write_error(&mut output, Some(request_id), &code, &message)?;
                }
                Message::Refresh {
                    request_id,
                    generation,
                } => {
                    if startup_pending {
                        refreshes.insert(request_id, (generation, 1));
                    } else {
                        scan_generation += 1;
                        discovery.scan(ScriptScan {
                            request_id: Some(request_id.clone()),
                            generation: scan_generation,
                            config: config.clone(),
                            configuration: false,
                        })?;
                        refreshes.insert(request_id, (generation, scan_generation));
                    }
                }
                Message::ConfigurationChanged {
                    request_id,
                    configuration,
                } => match ScriptConfig::from_configuration(&configuration) {
                    Ok(updated) => {
                        scan_generation += 1;
                        discovery.scan(ScriptScan {
                            request_id: Some(request_id.clone()),
                            generation: scan_generation,
                            config: updated.clone(),
                            configuration: true,
                        })?;
                        configurations.insert(request_id, updated);
                    }
                    Err(message) => write_error(
                        &mut output,
                        Some(request_id),
                        "invalid_configuration",
                        &message,
                    )?,
                },
                Message::Cancel {
                    request_id,
                    generation,
                } => {
                    if let Some((expected, scan)) = refreshes.get(&request_id)
                        && *expected == generation
                    {
                        discovery.cancel(*scan);
                    }
                }
                Message::PrepareEntries { .. } => {}
                message => write_error(
                    &mut output,
                    request_id(&message),
                    "unsupported_message",
                    "the script extension received an unsupported message",
                )?,
            },
            RuntimeEvent::CatalogUpdated { entry_ids } => {
                let current = scripts.read().unwrap_or_else(|error| error.into_inner());
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
            RuntimeEvent::ScanFinished { request_id, result } => {
                if let Some(id) = request_id {
                    if let Some(updated) = configurations.remove(&id) {
                        match result {
                            Ok(()) => {
                                config = updated;
                                write_frame(
                                    &mut output,
                                    &Message::ConfigurationApplied { request_id: id },
                                )?;
                            }
                            Err(message) => write_error(
                                &mut output,
                                Some(id),
                                "configuration_apply_failed",
                                &message,
                            )?,
                        }
                    } else if let Some((generation, _)) = refreshes.remove(&id) {
                        _write_refresh_result(&mut output, id, generation, &result)?;
                    }
                } else {
                    startup_pending = false;
                    if let Err(message) = &result {
                        eprintln!("script startup scan failed: {message}");
                    }
                    for (id, (generation, _)) in refreshes.drain() {
                        _write_refresh_result(&mut output, id, generation, &result)?;
                    }
                }
            }
            RuntimeEvent::ProtocolClosed => break,
            RuntimeEvent::ProtocolError(message) => {
                return Err(std::io::Error::other(message).into());
            }
        }
    }
    drop(discovery);
    Ok(())
}

fn _write_refresh_result(
    output: &mut impl std::io::Write,
    request_id: String,
    generation: u64,
    result: &Result<(), String>,
) -> Result<(), nanika_protocol::FrameError> {
    match result {
        Ok(()) => write_frame(
            output,
            &Message::Refreshed {
                request_id,
                generation,
            },
        ),
        Err(message) => write_error(output, Some(request_id), "discovery_failed", message),
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
        Message::Error { request_id, .. } => request_id.clone(),
        Message::CatalogApplied { .. }
        | Message::CandidatesChanged
        | Message::ViewInvalidated { .. }
        | Message::PrepareEntries { .. } => None,
    }
}

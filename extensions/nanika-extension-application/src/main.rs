//! Application extension process entry point.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, stdin, stdout};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};

use nanika_config::ConfigStore;
use nanika_extension_application::{
    ApplicationEntry, DiscoveryWorker, RuntimeEvent, RuntimePaths, select_candidates,
};
use nanika_protocol::{Message, PROTOCOL_NAME, read_frame, write_frame};

const EVENT_CAPACITY: usize = 8;
const MAX_CANDIDATES: usize = 5_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = RuntimePaths::resolve(std::env::args().skip(1))?;
    let config_store = ConfigStore::open(&paths.data_root, &paths.config_root)?;
    let database_path = paths.database_path();
    let icon_root = paths.icon_root();
    let entries = Arc::new(RwLock::new(Vec::<ApplicationEntry>::new()));
    let (event_sender, events) = mpsc::sync_channel(EVENT_CAPACITY);
    let _reader = spawn_protocol_reader(event_sender.clone())?;
    let worker = DiscoveryWorker::spawn(
        database_path,
        icon_root,
        config_store,
        Arc::clone(&entries),
        event_sender,
    )?;
    let mut output = BufWriter::new(stdout().lock());
    let mut initialized = false;
    let mut active_scans = 1_usize;
    let mut pending_query = None::<(String, u64, String)>;
    let mut refresh_requests = HashMap::<String, u64>::new();
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
                    let complete = active_scans == 0;
                    write_snapshot(
                        &mut output,
                        &entries,
                        &request_id,
                        generation,
                        &query,
                        complete,
                    )?;
                    pending_query = (!complete).then_some((request_id, generation, query));
                }
                Message::Refresh {
                    request_id,
                    generation,
                } => {
                    if !worker.refresh(Some(request_id.clone()), generation) {
                        write_error(
                            &mut output,
                            Some(request_id),
                            "refresh_queue_full",
                            "application refresh queue is full",
                        )?;
                    } else {
                        active_scans = active_scans.saturating_add(1);
                        refresh_requests.insert(request_id, generation);
                    }
                }
                Message::Cancel {
                    request_id,
                    generation,
                } => {
                    if pending_query
                        .as_ref()
                        .is_some_and(|(pending_id, pending_generation, _)| {
                            *pending_id == request_id && *pending_generation == generation
                        })
                    {
                        pending_query = None;
                    }
                    if refresh_requests.get(&request_id) == Some(&generation) {
                        worker.cancel(generation);
                    }
                }
                Message::Invoke { request_id, .. } => write_error(
                    &mut output,
                    Some(request_id),
                    "launch_service_unavailable",
                    "application launch requires the host launch service",
                )?,
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
            RuntimeEvent::ProtocolClosed => break,
            RuntimeEvent::ProtocolError(message) => {
                write_error(&mut output, None, "protocol_error", &message)?;
                break;
            }
            RuntimeEvent::ScanFinished {
                request_id: Some(request_id),
                response_generation,
                result: Ok(report),
            } if !report.cancelled => {
                active_scans = active_scans.saturating_sub(1);
                refresh_requests.remove(&request_id);
                complete_pending_query(&mut output, &entries, active_scans, &mut pending_query)?;
                write_frame(
                    &mut output,
                    &Message::Refreshed {
                        request_id,
                        generation: response_generation,
                    },
                )?;
            }
            RuntimeEvent::ScanFinished {
                request_id: Some(request_id),
                result: Err(message),
                ..
            } => {
                active_scans = active_scans.saturating_sub(1);
                refresh_requests.remove(&request_id);
                complete_pending_query(&mut output, &entries, active_scans, &mut pending_query)?;
                write_error(&mut output, Some(request_id), "refresh_failed", &message)?;
            }
            RuntimeEvent::ScanFinished {
                request_id: None,
                result: Err(message),
                ..
            } => {
                active_scans = active_scans.saturating_sub(1);
                complete_pending_query(&mut output, &entries, active_scans, &mut pending_query)?;
                eprintln!("application startup refresh failed: {message}");
            }
            RuntimeEvent::ScanFinished { request_id, .. } => {
                active_scans = active_scans.saturating_sub(1);
                if let Some(request_id) = request_id {
                    refresh_requests.remove(&request_id);
                }
                complete_pending_query(&mut output, &entries, active_scans, &mut pending_query)?;
            }
        }
    }
    worker.shutdown();
    Ok(())
}

fn complete_pending_query(
    output: &mut impl std::io::Write,
    entries: &RwLock<Vec<ApplicationEntry>>,
    active_scans: usize,
    pending_query: &mut Option<(String, u64, String)>,
) -> Result<(), nanika_protocol::FrameError> {
    if active_scans == 0
        && let Some((request_id, generation, query)) = pending_query.take()
    {
        write_snapshot(output, entries, &request_id, generation, &query, true)?;
    }
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
    let candidates = select_candidates(&entries, query, MAX_CANDIDATES);
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
        | Message::Cancel { request_id, .. }
        | Message::Refresh { request_id, .. }
        | Message::Refreshed { request_id, .. }
        | Message::Shutdown { request_id }
        | Message::ShutdownAck { request_id } => Some(request_id.clone()),
        Message::Initialized { request_id, .. } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
    }
}

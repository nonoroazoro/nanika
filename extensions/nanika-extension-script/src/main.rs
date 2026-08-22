//! Script extension process entry point.

use std::collections::BTreeMap;
use std::io::{BufReader, BufWriter, stdin, stdout};

use nanika_config::ConfigStore;
use nanika_extension_script::{RUN_ACTION_ID, RuntimePaths, ScriptConfig};
use nanika_protocol::{
    HostServiceRequest, HostServiceResponse, LaunchDescriptor, Message, PROTOCOL_NAME, read_frame,
    write_frame,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = RuntimePaths::parse(std::env::args().skip(1))?;
    let store = ConfigStore::open(&paths.data_root, &paths.config_root)?;
    let mut scripts = load_scripts(&store)?;
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
            } => write_frame(
                &mut output,
                &Message::Snapshot {
                    request_id,
                    generation,
                    complete: true,
                    entries: scripts
                        .values()
                        .take(5_000)
                        .map(|script| script.candidate())
                        .collect(),
                },
            )?,
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id,
            } => match scripts
                .get(&entry_id)
                .filter(|_| action_id == RUN_ACTION_ID)
                .map(|script| script.launch_descriptor())
            {
                Some(descriptor) => {
                    invoke_host(&mut input, &mut output, request_id, generation, descriptor)?
                }
                None => write_error(
                    &mut output,
                    Some(request_id),
                    "unknown_action",
                    "script entry or action does not exist",
                )?,
            },
            Message::Refresh {
                request_id,
                generation,
            } => match load_scripts(&store) {
                Ok(updated) => {
                    scripts = updated;
                    write_frame(
                        &mut output,
                        &Message::Refreshed {
                            request_id,
                            generation,
                        },
                    )?;
                }
                Err(message) => {
                    write_error(&mut output, Some(request_id), "refresh_failed", &message)?
                }
            },
            Message::Cancel { .. } => {}
            Message::Shutdown { request_id } => {
                write_frame(&mut output, &Message::ShutdownAck { request_id })?;
                break;
            }
            message => write_error(
                &mut output,
                request_id(&message),
                "unsupported_message",
                "the script extension received an unsupported message",
            )?,
        }
    }
    Ok(())
}

fn load_scripts(
    store: &ConfigStore,
) -> Result<BTreeMap<String, nanika_extension_script::ScriptEntry>, String> {
    Ok(ScriptConfig::load(store)?
        .scripts
        .into_iter()
        .map(|script| (format!("script.{}", script.id), script))
        .collect())
}

fn invoke_host(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    request_id: String,
    generation: u64,
    descriptor: LaunchDescriptor,
) -> Result<(), nanika_protocol::FrameError> {
    let service_request_id = format!("host-{request_id}");
    write_frame(
        output,
        &Message::HostRequest {
            request_id: service_request_id.clone(),
            parent_request_id: request_id.clone(),
            generation,
            request: HostServiceRequest::Launch { descriptor },
        },
    )?;
    loop {
        match read_frame(input)? {
            Some(Message::HostResponse {
                request_id: response_id,
                parent_request_id,
                generation: response_generation,
                response: HostServiceResponse::Launched,
            }) if response_id == service_request_id
                && parent_request_id == request_id
                && response_generation == generation =>
            {
                return write_frame(
                    output,
                    &Message::Result {
                        request_id,
                        generation,
                    },
                );
            }
            Some(Message::Error {
                request_id: Some(response_id),
                code,
                message,
            }) if response_id == service_request_id => {
                return write_error(output, Some(request_id), &code, &message);
            }
            Some(_) => {}
            None => return Ok(()),
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
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. }
        | Message::Shutdown { request_id }
        | Message::ShutdownAck { request_id } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
    }
}

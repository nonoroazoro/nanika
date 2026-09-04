//! Minimal child-process fixture for host supervisor tests.

use std::io::{self, stdin, stdout};

use nanika_protocol::{
    HostServiceRequest, HostServiceResponse, LaunchArguments, LaunchDescriptor, Message,
    PROTOCOL_NAME, read_frame, write_frame,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let hang_initialize = arguments
        .iter()
        .any(|argument| argument == "--hang-initialize");
    let exit_after_initialize = arguments
        .iter()
        .any(|argument| argument == "--exit-after-initialize");
    let delay_first_query = arguments
        .iter()
        .any(|argument| argument == "--delay-first-query");
    let incremental_query = arguments
        .iter()
        .any(|argument| argument == "--incremental-query");
    let crash_query_once = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--crash-query-once=")
            .map(std::path::PathBuf::from)
    });
    let hang_query_once = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--hang-query-once=")
            .map(std::path::PathBuf::from)
    });
    let hang_invoke = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--hang-invoke=")
            .map(std::path::PathBuf::from)
    });
    let mark_refresh = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--mark-refresh=")
            .map(std::path::PathBuf::from)
    });
    let request_launch_on_invoke = arguments
        .iter()
        .any(|argument| argument == "--request-launch-on-invoke");
    if arguments
        .iter()
        .any(|argument| argument == "--write-stderr")
    {
        eprint!("{}", "fixture-stderr-".repeat(1024));
    }
    let mut input = stdin().lock();
    let mut output = stdout().lock();
    let mut query_count = 0_u32;

    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Initialize { request_id, .. } => {
                if hang_initialize {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
                write_frame(
                    &mut output,
                    &Message::Initialized {
                        request_id,
                        protocol: PROTOCOL_NAME.to_owned(),
                    },
                )?;
                if exit_after_initialize {
                    return Ok(());
                }
            }
            Message::Shutdown { request_id } => {
                write_frame(&mut output, &Message::ShutdownAck { request_id })?;
                return Ok(());
            }
            Message::Query {
                request_id,
                generation,
                query,
            } => {
                query_count = query_count.saturating_add(1);
                if let Some(marker) = &crash_query_once
                    && !marker.exists()
                {
                    std::fs::write(marker, b"crashed")?;
                    return Ok(());
                }
                if let Some(marker) = &hang_query_once
                    && !marker.exists()
                {
                    std::fs::write(marker, b"hung")?;
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
                if delay_first_query && query_count == 1 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if incremental_query {
                    write_frame(
                        &mut output,
                        &Message::Snapshot {
                            request_id: request_id.clone(),
                            generation,
                            complete: false,
                            entries: vec![candidate("fixture.partial", "Partial")],
                        },
                    )?;
                }
                write_frame(
                    &mut output,
                    &Message::Snapshot {
                        request_id,
                        generation,
                        complete: true,
                        entries: vec![candidate(
                            "fixture.entry",
                            if query.is_empty() { "Fixture" } else { &query },
                        )],
                    },
                )?;
            }
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id,
            } => {
                if let Some(marker) = &hang_invoke {
                    std::fs::write(marker, b"hung")?;
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
                if request_launch_on_invoke
                    && entry_id == "fixture.entry"
                    && action_id == "fixture.run"
                {
                    let service_request_id = format!("host-{request_id}");
                    write_frame(
                        &mut output,
                        &Message::HostRequest {
                            request_id: service_request_id.clone(),
                            parent_request_id: request_id.clone(),
                            generation,
                            request: HostServiceRequest::Launch {
                                descriptor: LaunchDescriptor::Program {
                                    program: "fixture-program".to_owned(),
                                    arguments: LaunchArguments::default(),
                                    working_directory: None,
                                },
                            },
                        },
                    )?;
                    match read_frame(&mut input)? {
                        Some(Message::HostResponse {
                            request_id: response_id,
                            response: HostServiceResponse::Launched,
                            ..
                        }) if response_id == service_request_id => {}
                        Some(Message::Error { code, message, .. }) => {
                            write_frame(
                                &mut output,
                                &Message::Error {
                                    request_id: Some(request_id),
                                    code,
                                    message,
                                },
                            )?;
                            continue;
                        }
                        _ => continue,
                    }
                }
                let response = if entry_id == "fixture.entry" && action_id == "fixture.run" {
                    Message::Result {
                        request_id,
                        generation,
                        effect: nanika_protocol::NavigationEffect::Close,
                    }
                } else {
                    Message::Error {
                        request_id: Some(request_id),
                        code: "unknown_action".to_owned(),
                        message: "fixture entry or action does not exist".to_owned(),
                    }
                };
                write_frame(&mut output, &response)?;
            }
            Message::Cancel { .. } => {}
            Message::Refresh {
                request_id,
                generation,
            } => {
                if let Some(marker) = &mark_refresh {
                    std::fs::write(marker, b"refreshed")?;
                }
                write_frame(
                    &mut output,
                    &Message::Refreshed {
                        request_id,
                        generation,
                    },
                )?;
            }
            Message::GetSettings { request_id } => write_frame(
                &mut output,
                &Message::Settings {
                    request_id,
                    contribution: nanika_protocol::SettingsContribution {
                        title: "Fixture".to_owned(),
                        fields: Vec::new(),
                    },
                },
            )?,
            Message::UpdateSettings {
                request_id,
                updates,
            } if updates.is_empty() => write_frame(
                &mut output,
                &Message::SettingsUpdated {
                    request_id,
                    contribution: nanika_protocol::SettingsContribution {
                        title: "Fixture".to_owned(),
                        fields: Vec::new(),
                    },
                },
            )?,
            Message::Snapshot { .. }
            | Message::Result { .. }
            | Message::ViewEvent { .. }
            | Message::ViewUpdated { .. }
            | Message::ViewClose { .. }
            | Message::ViewClosed { .. }
            | Message::Refreshed { .. }
            | Message::HostRequest { .. }
            | Message::HostResponse { .. }
            | Message::Initialized { .. }
            | Message::ShutdownAck { .. }
            | Message::Settings { .. }
            | Message::SettingsUpdated { .. }
            | Message::UpdateSettings { .. }
            | Message::Error { .. } => write_frame(
                &mut output,
                &Message::Error {
                    request_id: None,
                    code: "unsupported_message".to_owned(),
                    message: "fixture received an unsupported message type".to_owned(),
                },
            )?,
        }
    }

    let _ = io::Write::flush(&mut output);
    Ok(())
}

fn candidate(entry_id: &str, title: &str) -> nanika_protocol::Candidate {
    nanika_protocol::Candidate {
        entry_id: entry_id.to_owned(),
        title: title.to_owned(),
        subtitle: Some("Fixture".to_owned()),
        action_id: "fixture.run".to_owned(),
        aliases: vec!["fixture alias".to_owned()],
        icon: None,
    }
}

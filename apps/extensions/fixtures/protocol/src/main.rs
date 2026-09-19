//! Minimal child-process fixture for host supervisor tests.

use std::io::{self, stdin, stdout};

use nanika_protocol::{
    HostServiceRequest, HostServiceResponse, LaunchArguments, LaunchDescriptor, Message,
    PROTOCOL_NAME, read_frame, write_frame,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let error_after_initialize = arguments
        .iter()
        .any(|argument| argument == "--error-after-initialize");
    let incremental_query = arguments
        .iter()
        .any(|argument| argument == "--incremental-query");
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
    let cancellation_query = arguments.iter().find_map(|value| {
        value
            .strip_prefix("--cancel-query=")
            .map(std::path::PathBuf::from)
    });
    let cancellation_invoke = arguments.iter().find_map(|value| {
        value
            .strip_prefix("--cancel-invoke=")
            .map(std::path::PathBuf::from)
    });
    let complete_on_cancel = arguments
        .iter()
        .any(|value| value == "--complete-on-cancel");
    let mut pending_query = None;
    let mut deferred_configuration = None;
    let mut pending_invoke = None;
    let mut previous_result = None;
    let mut open_views = std::collections::HashSet::new();
    let mut input = stdin().lock();
    let mut output = stdout().lock();

    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Initialize { request_id, .. } => {
                if let Some(root) = data_root(&arguments) {
                    std::fs::write(root.join(format!("{request_id}.entered")), b"initializing")?;
                }
                wait_for_release(&arguments, &request_id)?;
                write_frame(
                    &mut output,
                    &Message::Initialized {
                        request_id,
                        protocol: PROTOCOL_NAME.to_owned(),
                    },
                )?;
                if error_after_initialize {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: None,
                            code: "background_failure".to_owned(),
                            message: "fixture background operation failed".to_owned(),
                        },
                    )?;
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
                if let Some(marker) = &cancellation_query
                    && query == "blocked"
                {
                    std::fs::write(marker, b"query waiting")?;
                    pending_query = Some(request_id);
                    continue;
                }
                if let Some(root) = data_root(&arguments)
                    && let Some(extension_id) = request_id
                        .strip_prefix("search-")
                        .and_then(|id| id.rsplit_once('-').map(|(id, _)| id))
                    && root.join(format!("fail-search-{extension_id}")).exists()
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            code: "fixture_query_failure".to_owned(),
                            message: "fixture query failed with an internal cause".to_owned(),
                        },
                    )?;
                    continue;
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
                if let Some(marker) = &cancellation_invoke {
                    std::fs::write(marker, request_id.as_bytes())?;
                    if let Some(previous) = &previous_result {
                        // A duplicate old completion must never complete the next action.
                        write_frame(&mut output, previous)?;
                    }
                    pending_invoke = Some((request_id, generation));
                    continue;
                }
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
                let response = if entry_id == "fixture.view"
                    && action_id == nanika_protocol::VIEW_OPEN_ACTION_ID
                {
                    let view_id = format!("view-{request_id}");
                    open_views.insert(view_id.clone());
                    Message::Result {
                        request_id,
                        generation,
                        effect: nanika_protocol::NavigationEffect::Push {
                            view_id,
                            revision: 1,
                            view: Box::new(nanika_protocol::View::Detail {
                                detail: nanika_protocol::DetailView {
                                    title: None,
                                    content: nanika_protocol::DetailContent::Text {
                                        value: "Fixture view".to_owned(),
                                    },
                                    metadata: Vec::new(),
                                    actions: Vec::new(),
                                },
                            }),
                        },
                    }
                } else if entry_id == "fixture.entry"
                    && (action_id == "fixture.run"
                        || action_id == nanika_protocol::COMMAND_EXECUTE_ACTION_ID)
                {
                    Message::Result {
                        request_id,
                        generation,
                        effect: nanika_protocol::NavigationEffect::Dismiss,
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
            Message::ViewClose {
                request_id,
                view_id,
            } => {
                let response = if open_views.remove(&view_id) {
                    Message::ViewClosed {
                        request_id,
                        view_id,
                    }
                } else {
                    Message::Error {
                        request_id: Some(request_id),
                        code: "unknown_view".to_owned(),
                        message: "fixture view is not open".to_owned(),
                    }
                };
                write_frame(&mut output, &response)?;
            }
            Message::Cancel {
                request_id,
                generation,
            } => {
                if pending_query.as_ref() == Some(&request_id) {
                    pending_query = None;
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            code: "cancelled".to_owned(),
                            message: "superseded query cancelled".to_owned(),
                        },
                    )?;
                } else if pending_invoke.as_ref() == Some(&(request_id.clone(), generation)) {
                    pending_invoke = None;
                    let terminal = if complete_on_cancel {
                        Message::Result {
                            request_id,
                            generation,
                            effect: nanika_protocol::NavigationEffect::Dismiss,
                        }
                    } else {
                        Message::Error {
                            request_id: Some(request_id),
                            code: "cancelled".to_owned(),
                            message: "action cancelled".to_owned(),
                        }
                    };
                    if matches!(terminal, Message::Result { .. }) {
                        previous_result = Some(terminal.clone());
                    }
                    write_frame(&mut output, &terminal)?;
                }
            }
            Message::Refresh {
                request_id,
                generation,
            } => {
                wait_for_release(&arguments, &request_id)?;
                if arguments
                    .iter()
                    .any(|argument| argument == "--fail-refresh")
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            code: "refresh_failed".to_owned(),
                            message: "fixture refresh failed".to_owned(),
                        },
                    )?;
                    continue;
                }
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
            Message::ConfigurationChanged { request_id, .. } => {
                if request_id == "deferred-settings" {
                    if let Some(root) = data_root(&arguments) {
                        std::fs::write(root.join("deferred-settings.entered"), b"pending")?;
                    }
                    deferred_configuration = Some(request_id);
                    continue;
                }
                wait_for_release(&arguments, &request_id)?;
                if data_root(&arguments)
                    .is_some_and(|root| root.join(format!("fail-{request_id}")).exists())
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            code: "configuration_failed".to_owned(),
                            message: "fixture could not apply configuration".to_owned(),
                        },
                    )?;
                    continue;
                }
                write_frame(&mut output, &Message::ConfigurationApplied { request_id })?;
            }
            Message::PrepareEntries { .. } => {}
            Message::Snapshot { .. }
            | Message::CandidatesChanged
            | Message::ViewInvalidated { .. }
            | Message::Result { .. }
            | Message::ViewEvent { .. }
            | Message::ViewUpdated { .. }
            | Message::ViewClosed { .. }
            | Message::Refreshed { .. }
            | Message::HostRequest { .. }
            | Message::HostResponse { .. }
            | Message::Initialized { .. }
            | Message::ShutdownAck { .. }
            | Message::ConfigurationApplied { .. }
            | Message::Error { .. } => write_frame(
                &mut output,
                &Message::Error {
                    request_id: None,
                    code: "unsupported_message".to_owned(),
                    message: "fixture received an unsupported message type".to_owned(),
                },
            )?,
        }
        if let Some(request_id) = deferred_configuration.take() {
            write_frame(&mut output, &Message::ConfigurationApplied { request_id })?;
        }
    }

    let _ = io::Write::flush(&mut output);
    Ok(())
}

fn candidate(entry_id: &str, title: &str) -> nanika_protocol::Candidate {
    nanika_protocol::Candidate {
        kind: nanika_protocol::CandidateKind::Action,
        entry_id: entry_id.to_owned(),
        title: title.to_owned(),
        subtitle: Some("Fixture".to_owned()),
        action_id: "fixture.run".to_owned(),
        aliases: vec!["fixture alias".to_owned()],
        icon: None,
        contribution_icon: None,
    }
}

fn data_root(arguments: &[String]) -> Option<std::path::PathBuf> {
    arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--data-root=").map(Into::into))
}

fn wait_for_release(arguments: &[String], request_id: &str) -> io::Result<()> {
    let Some(root) = data_root(arguments) else {
        return Ok(());
    };
    let blocker = root.join(format!("{request_id}.block"));
    if blocker.exists() {
        std::fs::write(root.join(format!("{request_id}.entered")), b"waiting")?;
        while blocker.exists() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    Ok(())
}

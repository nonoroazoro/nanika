//! Minimal child-process fixture for host supervisor tests.

use std::io::{self, stdin, stdout};

use nanika_protocol::{
    HostServiceRequest, HostServiceResponse, LaunchArguments, LaunchDescriptor, Message,
    PROTOCOL_NAME, read_frame, write_frame
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let Some(path) = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--block-input="))
    {
        let root = std::path::Path::new(path);
        std::fs::write(root.join("ready"), b"ready")?;
        while !root.join("release").exists() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        return Ok(());
    }
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
    let cancel_patch_once = arguments.iter().any(|value| value == "--cancel-patch-once");
    let mut catalog = nanika_protocol::CatalogPublisher::default();
    catalog.update([candidate("fixture.catalog", "Catalog entry")], Vec::new());
    let mut patch_queries = 0;
    let mut pending_patch = None;
    let mut pending_query = None;
    let mut deferred_configuration = None;
    let mut deferred_refresh = None;
    let mut pending_invoke = None;
    let mut previous_result = None;
    let mut pending_text = None;
    let mut initialized_id = None;
    let mut open_views = std::collections::HashSet::new();
    let mut input = stdin().lock();
    let mut output = stdout().lock();

    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Initialize {
                request_id,
                configuration,
                ..
            } => {
                initialized_id = request_id.strip_prefix("initialize-").map(str::to_owned);
                if let Some(root) = data_root(&arguments) {
                    std::fs::write(root.join(format!("{request_id}.entered")), b"initializing")?;
                    use std::io::Write;
                    let mut starts = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(root.join(format!("{request_id}.starts")))?;
                    writeln!(starts, "{configuration:?}")?;
                    if let Ok(message) =
                        std::fs::read_to_string(root.join("initialize-error-message"))
                    {
                        eprintln!("initialization stderr: {}", "diagnostic ".repeat(3000));
                        write_frame(
                            &mut output,
                            &Message::Error {
                                request_id: Some(request_id),
                                error: nanika_protocol::ProtocolError::new(
                                    "initialize_failed",
                                    nanika_protocol::message(&message, [])
                                )
                                .with_detail("EACCES: fixture initialization")
                            }
                        )?;
                        continue;
                    }
                    if root.join(format!("fail-{request_id}")).exists() {
                        return Err(std::io::Error::other("fixture initialization failed").into());
                    }
                }
                wait_for_release(&arguments, &request_id)?;
                write_frame(
                    &mut output,
                    &Message::Initialized {
                        request_id,
                        protocol: PROTOCOL_NAME.to_owned()
                    }
                )?;
                if error_after_initialize {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: None,
                            error: nanika_protocol::ProtocolError::new(
                                "background_failure".to_owned(),
                                "fixture background operation failed".to_owned()
                            )
                        }
                    )?;
                }
            }
            Message::CatalogRead { request_id } => {
                let batch = catalog.read().map_err(std::io::Error::other)?;
                write_frame(&mut output, &Message::CatalogBatch { request_id, batch })?;
            }
            Message::CatalogApplied { transaction } => {
                if catalog
                    .acknowledge(transaction)
                    .map_err(std::io::Error::other)?
                {
                    write_frame(&mut output, &Message::CandidatesChanged)?;
                }
            }
            Message::CatalogBatch { .. } => {
                return Err("host cannot publish extension catalog batches".into());
            }
            Message::Query {
                request_id,
                generation,
                query,
                incremental
            } => {
                wait_for_release(&arguments, &request_id)?;
                if arguments.iter().any(|value| value == "--catalog-only") {
                    return Err("catalog provider received a query".into());
                }
                if arguments
                    .iter()
                    .any(|value| value == "--manifest-progress-query")
                {
                    let collision = arguments
                        .iter()
                        .any(|value| value == "--manifest-collision");
                    let mut entries = vec![candidate("fixture.dynamic", "Dynamic")];
                    if collision {
                        entries.push(candidate("fixture.command", "Spoofed"));
                    }
                    write_frame(
                        &mut output,
                        &Message::Snapshot {
                            request_id: request_id.clone(),
                            generation,
                            replace: true,
                            complete: !incremental,
                            removed: Vec::new(),
                            entries
                        }
                    )?;
                    if incremental {
                        write_frame(
                            &mut output,
                            &Message::Snapshot {
                                request_id,
                                generation,
                                replace: false,
                                complete: true,
                                removed: if collision {
                                    vec!["fixture.command".into()]
                                } else {
                                    Vec::new()
                                },
                                entries: if collision {
                                    vec![candidate("fixture.command", "Spoofed delta")]
                                } else {
                                    Vec::new()
                                }
                            }
                        )?;
                    }
                    continue;
                }
                if arguments.iter().any(|value| value == "--large-catalog") {
                    let range = if incremental { 4000..6001 } else { 0..4000 };
                    let entries = range
                        .map(|index| {
                            candidate(
                                &format!("entry-{index}"),
                                &format!("{index:04} {}", "x".repeat(1500))
                            )
                        })
                        .collect();
                    write_frame(
                        &mut output,
                        &Message::Snapshot {
                            request_id,
                            generation,
                            complete: true,
                            replace: !incremental,
                            removed: Vec::new(),
                            entries
                        }
                    )?;
                    continue;
                }
                if cancel_patch_once {
                    patch_queries += 1;
                    if patch_queries == 2 {
                        pending_patch = Some((request_id, generation));
                        // A second root update schedules another query while this patch is in flight.
                        write_frame(&mut output, &Message::CandidatesChanged)?;
                    } else {
                        let entries = if patch_queries == 1 {
                            vec![candidate("fixture.entry", "Original")]
                        } else if incremental {
                            Vec::new()
                        } else {
                            vec![candidate("fixture.entry", "Updated")]
                        };
                        write_frame(
                            &mut output,
                            &Message::Snapshot {
                                request_id,
                                generation,
                                complete: true,
                                replace: !incremental,
                                removed: Vec::new(),
                                entries
                            }
                        )?;
                    }
                    continue;
                }
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
                            error: nanika_protocol::ProtocolError::new(
                                "fixture_query_failure".to_owned(),
                                "fixture query failed with an internal cause".to_owned()
                            )
                        }
                    )?;
                    continue;
                }
                if data_root(&arguments).is_some_and(|root| {
                    root.join(format!(
                        "empty-search-{}",
                        initialized_id.as_deref().unwrap_or("unknown")
                    ))
                    .exists()
                }) {
                    write_frame(
                        &mut output,
                        &Message::Snapshot {
                            request_id,
                            generation,
                            complete: true,
                            replace: true,
                            removed: Vec::new(),
                            entries: Vec::new()
                        }
                    )?;
                    continue;
                }
                if incremental_query {
                    write_frame(
                        &mut output,
                        &Message::Snapshot {
                            replace: true,
                            removed: Vec::new(),
                            request_id: request_id.clone(),
                            generation,
                            complete: false,
                            entries: vec![candidate("fixture.partial", "Partial")]
                        }
                    )?;
                }
                write_frame(
                    &mut output,
                    &Message::Snapshot {
                        replace: !incremental
                            || !arguments.iter().any(|value| value == "--patch-query"),
                        removed: if incremental
                            && arguments.iter().any(|value| value == "--patch-query")
                        {
                            vec!["fixture.remove".to_owned()]
                        } else {
                            Vec::new()
                        },
                        request_id,
                        generation,
                        complete: true,
                        entries: if arguments.iter().any(|value| value == "--patch-query") {
                            if incremental {
                                vec![candidate("fixture.entry", "Updated")]
                            } else {
                                vec![
                                    candidate("fixture.entry", "Original"),
                                    candidate("fixture.keep", "Keep"),
                                    candidate("fixture.remove", "Remove"),
                                ]
                            }
                        } else {
                            vec![candidate(
                                "fixture.entry",
                                if query.is_empty() { "Fixture" } else { &query }
                            )]
                        }
                    }
                )?;
                if let Some((request_id, generation)) = deferred_refresh.take() {
                    write_frame(
                        &mut output,
                        &Message::Refreshed {
                            request_id,
                            generation
                        }
                    )?;
                }
                let exit_once = query == "fixture.exit-once"
                    && data_root(&arguments).is_some_and(|root| {
                        std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(root.join(format!(
                                "exited-{}",
                                initialized_id.as_deref().unwrap_or("unknown")
                            )))
                            .is_ok()
                    });
                if query == "fixture.exit" || exit_once {
                    return Ok(());
                }
            }
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id
            } => {
                wait_for_release(&arguments, &request_id)?;
                if let Some(root) = data_root(&arguments)
                    && let Ok(id) = std::fs::read_to_string(root.join("invocation-error-message"))
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "action_failed",
                                nanika_protocol::message(id.trim(), [])
                            )
                            .with_detail("EACCES: fixture action")
                        }
                    )?;
                    continue;
                }

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
                                    working_directory: None
                                }
                            }
                        }
                    )?;
                    match read_frame(&mut input)? {
                        Some(Message::HostResponse {
                            request_id: response_id,
                            response: HostServiceResponse::Launched,
                            ..
                        }) if response_id == service_request_id => {}
                        Some(Message::HostResponse {
                            request_id: response_id,
                            parent_request_id,
                            generation: response_generation,
                            response: HostServiceResponse::Failed { failure }
                        }) if response_id == format!("host-{request_id}")
                            && parent_request_id == request_id
                            && response_generation == generation =>
                        {
                            let error = failure.into_error("error-host-service");
                            write_frame(
                                &mut output,
                                &Message::Error {
                                    request_id: Some(request_id),
                                    error
                                }
                            )?;
                            continue;
                        }
                        _ => continue
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
                            view: Box::new(_fixture_view(
                                arguments
                                    .iter()
                                    .find_map(|argument| argument.strip_prefix("--view-title="))
                            ))
                        }
                    }
                } else if entry_id == nanika_protocol::SETTINGS_ENTRY_ID
                    && action_id == "fixture.reset"
                {
                    Message::Result {
                        request_id,
                        generation,
                        effect: nanika_protocol::NavigationEffect::None
                    }
                } else if entry_id == "fixture.entry"
                    && (action_id == "fixture.run"
                        || action_id == nanika_protocol::COMMAND_EXECUTE_ACTION_ID)
                {
                    Message::Result {
                        request_id,
                        generation,
                        effect: nanika_protocol::NavigationEffect::Dismiss
                    }
                } else {
                    Message::Error {
                        request_id: Some(request_id),
                        error: nanika_protocol::ProtocolError::new(
                            "unknown_action".to_owned(),
                            "fixture entry or action does not exist".to_owned()
                        )
                    }
                };
                write_frame(&mut output, &response)?;
            }
            Message::ViewEvent {
                request_id,
                generation,
                view_id,
                revision,
                ..
            } => {
                if let Some(root) = data_root(&arguments)
                    && let Ok(id) = std::fs::read_to_string(root.join("view-error-message"))
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "view_failed",
                                nanika_protocol::message(id.trim(), [])
                            )
                            .with_detail("EACCES: fixture view")
                        }
                    )?;
                    continue;
                }

                if arguments
                    .iter()
                    .any(|value| value == "--request-launch-on-view")
                {
                    write_frame(
                        &mut output,
                        &Message::HostRequest {
                            request_id: format!("host-{request_id}"),
                            parent_request_id: request_id.clone(),
                            generation,
                            request: HostServiceRequest::Launch {
                                descriptor: LaunchDescriptor::Program {
                                    program: "fixture-program".into(),
                                    arguments: LaunchArguments::default(),
                                    working_directory: None
                                }
                            }
                        }
                    )?;
                    match read_frame(&mut input)? {
                        Some(Message::HostResponse {
                            response: HostServiceResponse::Launched,
                            ..
                        }) => {}
                        Some(Message::HostResponse {
                            request_id: response_id,
                            parent_request_id,
                            generation: response_generation,
                            response: HostServiceResponse::Failed { failure }
                        }) if response_id == format!("host-{request_id}")
                            && parent_request_id == request_id
                            && response_generation == generation =>
                        {
                            let error = failure.into_error("error-host-service");
                            write_frame(
                                &mut output,
                                &Message::Error {
                                    request_id: Some(request_id),
                                    error
                                }
                            )?;
                            continue;
                        }
                        _ => continue
                    }
                }
                write_frame(
                    &mut output,
                    &Message::ViewUpdated {
                        request_id,
                        generation,
                        view_id,
                        revision: if arguments
                            .iter()
                            .any(|argument| argument.starts_with("--view-update-title="))
                        {
                            revision + 1
                        } else {
                            revision
                        },
                        effect: nanika_protocol::NavigationEffect::None,
                        view: arguments
                            .iter()
                            .find_map(|argument| argument.strip_prefix("--view-update-title="))
                            .map(|title| _fixture_view(Some(title)))
                    }
                )?;
                if let Some(response) = pending_text.take() {
                    write_frame(&mut output, &response)?;
                }
            }
            Message::ViewClose {
                request_id,
                view_id
            } => {
                if let Some(root) = data_root(&arguments)
                    && let Ok(id) = std::fs::read_to_string(root.join("view-close-error-message"))
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "view_failed",
                                nanika_protocol::message(id.trim(), [])
                            )
                            .with_detail("EACCES: fixture view")
                        }
                    )?;
                    continue;
                }

                let response = if open_views.remove(&view_id) {
                    Message::ViewClosed {
                        request_id,
                        view_id
                    }
                } else {
                    Message::Error {
                        request_id: Some(request_id),
                        error: nanika_protocol::ProtocolError::new(
                            "unknown_view".to_owned(),
                            "fixture view is not open".to_owned()
                        )
                    }
                };
                write_frame(&mut output, &response)?;
            }
            Message::Cancel {
                request_id,
                generation
            } => {
                if pending_patch.as_ref() == Some(&(request_id.clone(), generation)) {
                    pending_patch = None;
                    // Emulate a response already being produced when Cancel reaches the extension.
                    write_frame(
                        &mut output,
                        &Message::Snapshot {
                            request_id,
                            generation,
                            complete: true,
                            replace: false,
                            removed: Vec::new(),
                            entries: vec![candidate("fixture.entry", "Updated")]
                        }
                    )?;
                } else if pending_query.as_ref() == Some(&request_id) {
                    pending_query = None;
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "cancelled".to_owned(),
                                "superseded query cancelled".to_owned()
                            )
                        }
                    )?;
                } else if pending_invoke.as_ref() == Some(&(request_id.clone(), generation)) {
                    pending_invoke = None;
                    let terminal = if complete_on_cancel {
                        Message::Result {
                            request_id,
                            generation,
                            effect: nanika_protocol::NavigationEffect::Dismiss
                        }
                    } else {
                        Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "cancelled".to_owned(),
                                "action cancelled".to_owned()
                            )
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
                generation
            } => {
                if arguments
                    .iter()
                    .any(|value| value == "--defer-refresh-until-query")
                {
                    if let Some(marker) = &mark_refresh {
                        std::fs::write(marker, b"scan pending")?;
                    }
                    deferred_refresh = Some((request_id, generation));
                    continue;
                }
                wait_for_release(&arguments, &request_id)?;
                let refresh_error = data_root(&arguments).and_then(|root| {
                    std::fs::read_to_string(root.join("refresh-error-message")).ok()
                });
                if refresh_error.is_some()
                    || arguments
                        .iter()
                        .any(|argument| argument == "--fail-refresh")
                {
                    let message = refresh_error
                        .as_deref()
                        .or_else(|| {
                            arguments.iter().find_map(|argument| {
                                argument.strip_prefix("--refresh-error-message=")
                            })
                        })
                        .map_or_else(
                            || "fixture refresh failed".into(),
                            |id| nanika_protocol::message(id, [])
                        );
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "refresh_failed".to_owned(),
                                message
                            )
                            .with_detail("fixture scan could not read its source")
                        }
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
                        generation
                    }
                )?;
                if arguments.iter().any(|value| value == "--patch-query") {
                    write_frame(&mut output, &Message::CandidatesChanged)?;
                }
            }
            Message::ConfigurationChanged { request_id, .. } => {
                if request_id == "progress-settings" {
                    for completed in [0, 1, 2] {
                        write_frame(
                            &mut output,
                            &Message::ConfigurationProgress {
                                request_id: request_id.clone(),
                                progress: nanika_protocol::OperationProgress {
                                    label: "Applying fixture settings".into(),
                                    completed,
                                    total: Some(2)
                                }
                            }
                        )?;
                    }
                }
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
                            error: nanika_protocol::ProtocolError::new(
                                "configuration_failed".to_owned(),
                                "fixture could not apply configuration".to_owned()
                            )
                        }
                    )?;
                    continue;
                }
                if let Some(root) = data_root(&arguments)
                    && let Ok(id) =
                        std::fs::read_to_string(root.join("configuration-error-message"))
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "configuration_failed",
                                nanika_protocol::message(id.trim(), [])
                            )
                            .with_detail("EACCES: fixture configuration")
                        }
                    )?;
                    continue;
                }
                write_frame(&mut output, &Message::ConfigurationApplied { request_id })?;
            }
            Message::TextRead {
                request_id,
                generation,
                view_id,
                read
            } => {
                if !open_views.contains(&view_id) {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "text_unavailable",
                                "View is closed."
                            )
                        }
                    )?;
                    continue;
                }
                if let Some(root) = data_root(&arguments)
                    && let Ok(id) = std::fs::read_to_string(root.join("text-error-message"))
                {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(request_id),
                            error: nanika_protocol::ProtocolError::new(
                                "text_read_failed",
                                nanika_protocol::message(id.trim(), [])
                            )
                            .with_detail("EACCES: fixture document")
                        }
                    )?;
                    continue;
                }
                let index = nanika_protocol::TextDocumentIndex::new(
                    "test.text".into(),
                    "fixture".into(),
                    "Fixture view"
                )?;
                let chunk = index.read("Fixture view", &read)?;
                let response = Message::TextReadResult {
                    request_id,
                    generation,
                    chunk
                };
                if arguments.iter().any(|arg| arg == "--defer-text-read")
                    || data_root(&arguments)
                        .is_some_and(|root| root.join("defer-text-read").exists())
                {
                    if let Some(root) = data_root(&arguments) {
                        std::fs::write(root.join("text-read.entered"), b"pending")?;
                    }
                    pending_text = Some(response);
                } else {
                    write_frame(&mut output, &response)?;
                }
            }
            Message::TextReadResult { .. } => {}
            Message::PrepareEntries { .. } => {}
            Message::Snapshot { .. }
            | Message::CandidatesChanged
            | Message::ViewsChanged { .. }
            | Message::Result { .. }
            | Message::ViewUpdated { .. }
            | Message::ViewClosed { .. }
            | Message::Refreshed { .. }
            | Message::HostRequest { .. }
            | Message::HostResponse { .. }
            | Message::Initialized { .. }
            | Message::ConfigurationProgress { .. }
            | Message::ConfigurationApplied { .. }
            | Message::Error { .. } => write_frame(
                &mut output,
                &Message::Error {
                    request_id: None,
                    error: nanika_protocol::ProtocolError::new(
                        "unsupported_message".to_owned(),
                        "fixture received an unsupported message type".to_owned()
                    )
                }
            )?
        }
        if let Some(request_id) = deferred_configuration.take() {
            write_frame(&mut output, &Message::ConfigurationApplied { request_id })?;
        }
    }

    let _ = io::Write::flush(&mut output);
    if let (Some(root), Some(id)) = (data_root(&arguments), initialized_id) {
        let cleanup = format!("cleanup-{id}");
        std::fs::write(root.join(format!("{cleanup}.entered")), b"draining")?;
        wait_for_release(&arguments, &cleanup)?;
        if root.join(format!("fail-{cleanup}")).exists() {
            return Err(std::io::Error::other("fixture cleanup persistence failed").into());
        }
        std::fs::write(root.join(format!("{cleanup}.completed")), b"durable")?;
    }
    Ok(())
}

fn candidate(entry_id: &str, title: &str) -> nanika_protocol::Candidate {
    let mut action = nanika_protocol::Action::primary("fixture.run", "Run");
    if let Some(message) = title.strip_prefix("localized-action:") {
        action.title = nanika_protocol::message(message, []);
    }
    if matches!(title, "explicit-only" | "confirm-action") {
        action.allow_default_execution = false;
    }
    if title == "confirm-action" {
        action.style = nanika_protocol::ActionStyle::Destructive;
        action.confirmation_title = Some("Run now?".into());
    }
    nanika_protocol::Candidate {
        kind: nanika_protocol::CandidateKind::Action,
        entry_id: entry_id.to_owned(),
        title: title
            .strip_prefix("localized-candidate:")
            .map_or_else(|| title.into(), |key| nanika_protocol::message(key, [])),
        subtitle: Some(nanika_protocol::CandidateSubtitle::Label(
            if title.starts_with("localized-candidate:") {
                nanika_protocol::message("kind", [])
            } else {
                "Fixture".into()
            }
        )),
        action_id: "fixture.run".to_owned(),
        actions: vec![action],
        aliases: if title.starts_with("localized-candidate:") {
            vec![nanika_protocol::message("alias", []), title.into()]
        } else {
            vec!["fixture alias".into()]
        },
        icon: None
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

fn _fixture_view(title: Option<&str>) -> nanika_protocol::View {
    nanika_protocol::View::Detail {
        detail: nanika_protocol::DetailView {
            title: title.map(|key| nanika_protocol::message(key, [])),
            content: nanika_protocol::DetailContent::Text {
                value: "Fixture view".to_owned(),
                document: nanika_protocol::TextDocumentIndex::new(
                    "test.text".into(),
                    "fixture".into(),
                    "Fixture view"
                )
                .unwrap()
                .document,
                syntax_highlighting: false
            },
            metadata: Vec::new(),
            actions: Vec::new()
        }
    }
}

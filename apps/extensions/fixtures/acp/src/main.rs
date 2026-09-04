use std::future;
use std::io::Write;
use std::process::Stdio as ProcessStdio;
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest,
    InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
    SessionNotification, SessionUpdate, StopReason,
};
use agent_client_protocol::{Agent, Result, Stdio};

#[path = "DummyAgentState.rs"]
mod dummy_agent_state;

use dummy_agent_state::DummyAgentState;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(marker) = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--write-marker-after=")
            .map(str::to_owned)
    }) {
        std::thread::sleep(Duration::from_millis(500));
        if let Err(error) = std::fs::write(marker, b"orphaned") {
            eprintln!("ACP dummy helper failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(marker) = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--append-start-marker=")
            .map(str::to_owned)
    }) && let Err(error) = append_start_marker(&marker)
    {
        eprintln!("ACP dummy startup marker failed: {error}");
        std::process::exit(1);
    }
    if let Some(markers) = arguments.iter().find_map(|argument| {
        argument
            .strip_prefix("--spawn-child-at-start=")
            .map(str::to_owned)
    }) && let Some((started, descendant)) = markers.split_once('|')
        && let Err(error) = spawn_marker_child(started, descendant)
    {
        eprintln!("ACP dummy startup child failed: {error}");
        std::process::exit(1);
    }
    if let Err(error) = async_io::block_on(run()) {
        eprintln!("ACP dummy extension failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let state = Arc::new(DummyAgentState::default());
    let new_session_state = Arc::clone(&state);
    let prompt_state = Arc::clone(&state);
    let cancel_state = Arc::clone(&state);
    Agent
        .builder()
        .name("nanika-acp-dummy")
        .on_receive_request(
            async move |_request: InitializeRequest, responder, _connection| {
                responder.respond(
                    InitializeResponse::new(ProtocolVersion::V1)
                        .agent_capabilities(AgentCapabilities::new()),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: NewSessionRequest, responder, _connection| {
                responder.respond(NewSessionResponse::new(new_session_state.create_session()))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, connection| {
                if !prompt_state.contains(&request.session_id) {
                    return responder.respond_with_error(
                        agent_client_protocol::util::internal_error("unknown ACP session"),
                    );
                }
                let prompt = request.prompt.iter().find_map(|content| match content {
                    ContentBlock::Text(content) => Some(content.text.as_str()),
                    _ => None,
                });
                if prompt == Some("hang") {
                    return future::pending().await;
                }
                if let Some(marker) = prompt.and_then(|prompt| prompt.strip_prefix("hang-after|")) {
                    std::fs::write(marker, b"started").map_err(|error| {
                        agent_client_protocol::util::internal_error(error.to_string())
                    })?;
                    return future::pending().await;
                }
                if let Some(marker) = prompt.and_then(|prompt| prompt.strip_prefix("mark|")) {
                    std::fs::write(marker, b"completed").map_err(|error| {
                        agent_client_protocol::util::internal_error(error.to_string())
                    })?;
                }
                if let Some(arguments) =
                    prompt.and_then(|prompt| prompt.strip_prefix("spawn-child|"))
                    && let Some((started, descendant)) = arguments.split_once('|')
                {
                    spawn_marker_child(started, descendant).map_err(|error| {
                        agent_client_protocol::util::internal_error(error.to_string())
                    })?;
                    return future::pending().await;
                }
                connection.send_notification(SessionNotification::new(
                    request.session_id,
                    SessionUpdate::AgentMessageChunk(ContentChunk::new("Hello World".into())),
                ))?;
                responder.respond(PromptResponse::new(StopReason::EndTurn))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |notification: CancelNotification, _connection| {
                if cancel_state.contains(&notification.session_id) {
                    Ok(())
                } else {
                    Err(agent_client_protocol::util::internal_error(
                        "unknown ACP session",
                    ))
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}

fn append_start_marker(path: &str) -> std::io::Result<()> {
    let mut marker = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    marker.write_all(b"started\n")
}

fn spawn_marker_child(started: &str, descendant: &str) -> std::io::Result<()> {
    std::fs::write(started, b"started")?;
    let mut child = std::process::Command::new(std::env::current_exe()?);
    child
        .arg(format!("--write-marker-after={descendant}"))
        .stdin(ProcessStdio::null())
        .stdout(ProcessStdio::null())
        .stderr(ProcessStdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        child.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    child.spawn()?;
    Ok(())
}

use std::future;
use std::sync::Arc;

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
                if request.prompt.iter().any(|content| {
                    matches!(content, ContentBlock::Text(content) if content.text == "hang")
                }) {
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

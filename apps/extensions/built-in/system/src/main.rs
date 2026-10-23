use nanika_extension_system::action_for_entry;
use nanika_protocol::{
    COMMAND_EXECUTE_ACTION_ID, FrameError, HostServiceRequest, HostServiceResponse, Message,
    NavigationEffect, PROTOCOL_NAME, read_frame, write_frame
};
use std::io::{BufReader, BufWriter, stdin, stdout};
#[path = "PendingAction.rs"]
mod pending_action;
use pending_action::PendingAction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = BufReader::new(stdin().lock());
    let mut output = BufWriter::new(stdout().lock());
    let mut initialized = false;
    let mut pending: Option<PendingAction> = None;
    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Initialize {
                request_id,
                protocol,
                ..
            } => {
                if initialized {
                    _error(
                        &mut output,
                        Some(request_id),
                        "already_initialized",
                        "error-already-initialized"
                    )?;
                } else if protocol != PROTOCOL_NAME {
                    _error(
                        &mut output,
                        Some(request_id),
                        "unsupported_protocol",
                        "error-unsupported-protocol"
                    )?;
                } else {
                    initialized = true;
                    write_frame(
                        &mut output,
                        &Message::Initialized {
                            request_id,
                            protocol
                        }
                    )?;
                }
            }
            message if !initialized => _error(
                &mut output,
                _request_id(&message),
                "not_initialized",
                "error-not-initialized"
            )?,
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id
            } => {
                if pending.is_some() {
                    _error(&mut output, Some(request_id), "busy", "error-busy")?;
                    continue;
                }
                let Some(action) =
                    action_for_entry(&entry_id).filter(|_| action_id == COMMAND_EXECUTE_ACTION_ID)
                else {
                    _error(
                        &mut output,
                        Some(request_id),
                        "unknown_action",
                        "error-unknown-action"
                    )?;
                    continue;
                };
                let service_id = format!("system-{request_id}");
                write_frame(
                    &mut output,
                    &Message::HostRequest {
                        request_id: service_id.clone(),
                        parent_request_id: request_id.clone(),
                        generation,
                        request: HostServiceRequest::SystemAction { action }
                    }
                )?;
                pending = Some(PendingAction {
                    request_id,
                    service_id,
                    generation
                });
            }
            Message::HostResponse {
                request_id,
                parent_request_id,
                generation,
                response
            } => {
                let Some(current) = pending.as_ref() else {
                    return Err("Unexpected system service response.".into());
                };
                if current.service_id != request_id
                    || current.request_id != parent_request_id
                    || current.generation != generation
                {
                    return Err(
                        "System service response authority does not match the pending action."
                            .into()
                    );
                }
                let current = pending.take().expect("pending action was validated");
                if let HostServiceResponse::Failed { failure } = response {
                    write_frame(
                        &mut output,
                        &Message::Error {
                            request_id: Some(current.request_id),
                            error: failure.into_error("error-host-service")
                        }
                    )?;
                    continue;
                }
                let HostServiceResponse::SystemActionSubmitted = response else {
                    _error(
                        &mut output,
                        Some(current.request_id),
                        "invalid_response",
                        "error-invalid-response"
                    )?;
                    continue;
                };
                write_frame(
                    &mut output,
                    &Message::Result {
                        request_id: current.request_id,
                        generation,
                        effect: NavigationEffect::Dismiss
                    }
                )?;
            }
            Message::Error { .. } => {
                return Err("Unexpected host error outside a service response.".into());
            }
            // Host admission decides cancellation. Once a native operation was admitted,
            // retain the pending request until its real response, including after Cancel.
            Message::Cancel { .. } | Message::PrepareEntries { .. } => {}
            Message::Refresh {
                request_id,
                generation
            } => write_frame(
                &mut output,
                &Message::Refreshed {
                    request_id,
                    generation
                }
            )?,
            Message::ConfigurationChanged { request_id, .. } => {
                write_frame(&mut output, &Message::ConfigurationApplied { request_id })?
            }
            message => _error(
                &mut output,
                _request_id(&message),
                "unsupported_message",
                "error-unsupported-message"
            )?
        }
    }
    Ok(())
}

fn _error(
    output: &mut impl std::io::Write,
    request_id: Option<String>,
    code: &str,
    reference: &str
) -> Result<(), FrameError> {
    write_frame(
        output,
        &Message::Error {
            request_id,
            error: nanika_protocol::ProtocolError::new(
                code,
                nanika_protocol::message(reference, [])
            )
        }
    )
}

fn _request_id(message: &Message) -> Option<String> {
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
        | Message::TextRead { request_id, .. }
        | Message::TextReadResult { request_id, .. }
        | Message::Cancel { request_id, .. }
        | Message::Refresh { request_id, .. }
        | Message::Refreshed { request_id, .. }
        | Message::ConfigurationChanged { request_id, .. }
        | Message::ConfigurationProgress { request_id, .. }
        | Message::ConfigurationApplied { request_id }
        | Message::HostRequest { request_id, .. }
        | Message::HostResponse { request_id, .. } => Some(request_id.clone()),
        Message::Error { request_id, .. } => request_id.clone(),
        _ => None
    }
}

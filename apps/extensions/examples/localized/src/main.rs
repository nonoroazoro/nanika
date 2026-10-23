//! A read-only external extension. Its runtime depends only on the wire protocol.
use nanika_protocol::{
    Message, NavigationEffect, PROTOCOL_NAME, VIEW_OPEN_ACTION_ID, ViewEvent, read_frame,
    write_frame
};
use std::io::{stdin, stdout};

mod view;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = stdin().lock();
    let mut output = stdout().lock();
    let Some(Message::Initialize {
        request_id,
        protocol,
        ..
    }) = read_frame(&mut input)?
    else {
        return Err("Expected initialization before other messages".into());
    };
    if protocol != PROTOCOL_NAME {
        return Err("Unsupported extension protocol".into());
    }
    write_frame(
        &mut output,
        &Message::Initialized {
            request_id,
            protocol
        }
    )?;
    while let Some(request) = read_frame(&mut input)? {
        let response = match request {
            Message::Invoke {
                request_id,
                generation,
                entry_id,
                action_id
            } if entry_id == "example.files" && action_id == VIEW_OPEN_ACTION_ID => {
                Message::Result {
                    request_id,
                    generation,
                    effect: NavigationEffect::Push {
                        view_id: "example.files".into(),
                        revision: 1,
                        view: Box::new(view::files())
                    }
                }
            }
            Message::ViewEvent {
                request_id,
                generation,
                view_id,
                revision,
                event
            } if view_id == "example.files"
                && revision == 1
                && matches!(event, ViewEvent::Resumed | ViewEvent::Invalidated) =>
            {
                Message::ViewUpdated {
                    request_id,
                    generation,
                    view_id,
                    revision,
                    effect: NavigationEffect::None,
                    view: None
                }
            }
            Message::ViewClose {
                request_id,
                view_id
            } => Message::ViewClosed {
                request_id,
                view_id
            },
            Message::Refresh {
                request_id,
                generation
            } => Message::Refreshed {
                request_id,
                generation
            },
            Message::ConfigurationChanged { request_id, .. } => {
                Message::ConfigurationApplied { request_id }
            }
            Message::PrepareEntries { .. } | Message::Cancel { .. } => continue,
            _ => return Err("Unexpected request for the read-only example".into())
        };
        write_frame(&mut output, &response)?;
    }
    Ok(())
}

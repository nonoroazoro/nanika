use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering}
};

use crate::SupervisorError;
use nanika_protocol::{FrameError, Message, TextChunk, TextRead};

pub(crate) type TextReadCompletion = Box<dyn FnOnce(Result<TextChunk, SupervisorError>) + Send>;

/// A replaceable read slot. Background document responses never occupy the
/// query/view receiver, including late responses to superseded reads.
#[derive(Default)]
pub(crate) struct TextReadReply {
    _pending: Mutex<Option<crate::PendingTextRead>>,
    _closed: AtomicBool
}

impl TextReadReply {
    pub(crate) fn register(
        &self,
        id: String,
        generation: u64,
        read: TextRead,
        resources: Option<std::sync::Arc<nanika_localization::ResourceSet>>,
        completion: TextReadCompletion
    ) -> bool {
        let mut pending = self
            ._pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self._closed.load(Ordering::Acquire) {
            drop(pending);
            completion(Err(SupervisorError::ChannelClosed));
            return false;
        }
        let previous = pending.replace(crate::PendingTextRead {
            id,
            generation,
            read,
            resources,
            completion
        });
        drop(pending);
        if let Some(previous) = previous {
            (previous.completion)(Err(SupervisorError::Cancelled("superseded text read")));
        }
        true
    }

    pub(crate) fn fail(&self, error: SupervisorError) {
        self._closed.store(true, Ordering::Release);
        let pending = self
            ._pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(pending) = pending {
            (pending.completion)(Err(error));
        }
    }

    pub(crate) fn dispatch(&self, frame: &Result<Option<Message>, FrameError>) -> bool {
        if !matches!(frame, Ok(Some(_))) {
            self._closed.store(true, Ordering::Release);
        }
        let mut pending = self
            ._pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(current) = pending.as_ref() else {
            return _is_text_reply(frame);
        };
        let (result, consumed) = match frame {
            Ok(Some(Message::TextReadResult {
                request_id,
                generation: actual,
                chunk
            })) if &current.id == request_id => {
                let result = if &current.generation != actual {
                    Err(SupervisorError::UnexpectedMessage(
                        "Text read generation does not match.".to_owned()
                    ))
                } else {
                    chunk
                        .validate_for(&current.read)
                        .map(|()| chunk.clone())
                        .map_err(SupervisorError::UnexpectedMessage)
                };
                (result, true)
            }
            Ok(Some(Message::Error { request_id, error }))
                if request_id.as_ref().is_none_or(|id| id == &current.id) =>
            {
                (
                    Err(crate::prepare_extension_error(
                        error.clone(),
                        current.resources.as_deref()
                    )),
                    request_id.is_some()
                )
            }
            Ok(None) => (Err(SupervisorError::ChannelClosed), false),
            Err(error) => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "Text read transport failed: {error}"
                ))),
                false
            ),
            _ => return _is_text_reply(frame)
        };
        let completion = pending.take().expect("pending text read").completion;
        drop(pending);
        completion(result);
        consumed
    }
}

fn _is_text_reply(frame: &Result<Option<Message>, FrameError>) -> bool {
    matches!(frame, Ok(Some(Message::TextReadResult { .. })))
        || matches!(frame, Ok(Some(Message::Error { request_id: Some(id), .. })) if id.starts_with("text-read-"))
}

#[cfg(test)]
#[path = "../tests/TextReadReply.rs"]
mod tests;

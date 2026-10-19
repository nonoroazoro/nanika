use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering}
};

use nanika_protocol::{FrameError, Message, TextChunk, TextRead};

pub(crate) type TextReadCompletion = Box<dyn FnOnce(Result<TextChunk, String>) + Send>;

/// A replaceable read slot. Background document responses never occupy the
/// query/view receiver, including late responses to superseded reads.
#[derive(Default)]
pub(crate) struct TextReadReply {
    _pending: Mutex<Option<(String, u64, TextRead, TextReadCompletion)>>,
    _closed: AtomicBool
}

impl TextReadReply {
    pub(crate) fn register(
        &self,
        id: String,
        generation: u64,
        read: TextRead,
        completion: TextReadCompletion
    ) -> bool {
        let mut pending = self
            ._pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self._closed.load(Ordering::Acquire) {
            drop(pending);
            completion(Err("Text source connection is closed.".to_owned()));
            return false;
        }
        let previous = pending.replace((id, generation, read, completion));
        drop(pending);
        if let Some((_, _, _, completion)) = previous {
            completion(Err("Text read was superseded.".to_owned()));
        }
        true
    }

    pub(crate) fn fail(&self, message: String) {
        self._closed.store(true, Ordering::Release);
        let pending = self
            ._pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some((_, _, _, completion)) = pending {
            completion(Err(message));
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
        let Some((id, generation, read, _)) = pending.as_ref() else {
            return _is_text_reply(frame);
        };
        let (result, consumed) = match frame {
            Ok(Some(Message::TextReadResult {
                request_id,
                generation: actual,
                chunk
            })) if id == request_id => {
                let result = if generation != actual {
                    Err("Text read generation does not match.".to_owned())
                } else {
                    chunk.validate_for(read).map(|()| chunk.clone())
                };
                (result, true)
            }
            Ok(Some(Message::Error {
                request_id: Some(request_id),
                code,
                message
            })) if id == request_id => (Err(format!("Text read failed: {code}: {message}")), true),
            Ok(Some(Message::Error {
                request_id: None,
                code,
                message
            })) => (Err(format!("Text read failed: {code}: {message}")), false),
            Ok(None) => (Err("Extension closed during text read.".to_owned()), false),
            Err(error) => (Err(format!("Text read transport failed: {error}")), false),
            _ => return _is_text_reply(frame)
        };
        let (_, _, _, completion) = pending.take().expect("pending text read");
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

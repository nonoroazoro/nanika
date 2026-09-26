use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use crate::SupervisorError;
use nanika_protocol::{FrameError, Message};

pub(crate) type RefreshCompletion = Box<dyn FnOnce(Result<(), SupervisorError>) + Send>;

/// Routes scan completion independently of query and action replies.
#[derive(Default)]
pub(crate) struct RefreshReply {
    _pending: Mutex<Option<(String, u64, RefreshCompletion)>>,
    _closed: AtomicBool,
}

impl RefreshReply {
    pub(crate) fn register(
        &self,
        id: String,
        generation: u64,
        completion: RefreshCompletion,
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
        if pending.is_some() {
            drop(pending);
            completion(Err(SupervisorError::UnexpectedMessage(
                "refresh is already pending".to_owned(),
            )));
            return false;
        }
        *pending = Some((id, generation, completion));
        true
    }

    pub(crate) fn fail(&self, error: SupervisorError) {
        self._closed.store(true, Ordering::Release);
        let pending = self
            ._pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some((_, _, completion)) = pending {
            completion(Err(error));
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
        let Some((id, expected_generation, _)) = pending.as_ref() else {
            return false;
        };
        let (result, consumed) = match frame {
            Ok(Some(Message::Refreshed {
                request_id,
                generation,
            })) if request_id == id && generation == expected_generation => (Ok(()), true),
            Ok(Some(Message::Error {
                request_id: Some(request_id),
                code,
                message,
            })) if request_id == id => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "refresh failed: {code}: {message}"
                ))),
                true,
            ),
            Ok(Some(Message::Error {
                request_id: None,
                code,
                message,
            })) => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "refresh failed: {code}: {message}"
                ))),
                false,
            ),
            Ok(None) => (Err(SupervisorError::ChannelClosed), false),
            Err(error) => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "refresh transport failed: {error}"
                ))),
                false,
            ),
            _ => return false,
        };
        let (_, _, completion) = pending.take().expect("pending refresh reply");
        drop(pending);
        completion(result);
        consumed
    }
}

#[cfg(test)]
#[path = "../tests/RefreshReply.rs"]
mod tests;

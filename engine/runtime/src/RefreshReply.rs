use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering}
};

use crate::SupervisorError;
use nanika_protocol::{FrameError, Message};

pub(crate) type RefreshCompletion = Box<dyn FnOnce(Result<(), SupervisorError>) + Send>;

/// Routes scan completion independently of query and action replies.
#[derive(Default)]
pub(crate) struct RefreshReply {
    _pending: Mutex<Option<crate::PendingRefresh>>,
    _closed: AtomicBool
}

impl RefreshReply {
    pub(crate) fn register(
        &self,
        id: String,
        generation: u64,
        resources: Option<std::sync::Arc<nanika_localization::ResourceSet>>,
        completion: RefreshCompletion
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
                "refresh is already pending".to_owned()
            )));
            return false;
        }
        *pending = Some(crate::PendingRefresh {
            id,
            generation,
            resources,
            completion
        });
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
            return false;
        };
        let (result, consumed) = match frame {
            Ok(Some(Message::Refreshed {
                request_id,
                generation
            })) if request_id == &current.id && generation == &current.generation => (Ok(()), true),
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
                    "refresh transport failed: {error}"
                ))),
                false
            ),
            _ => return false
        };
        let completion = pending.take().expect("pending refresh reply").completion;
        drop(pending);
        completion(result);
        consumed
    }
}

#[cfg(test)]
#[path = "../tests/RefreshReply.rs"]
mod tests;

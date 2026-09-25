use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use nanika_protocol::{FrameError, Message};

use crate::SupervisorError;

pub(crate) type ConfigurationCompletion = Box<dyn FnOnce(Result<(), SupervisorError>) + Send>;

/// One in-flight configuration reply, dispatched independently of search responses.
#[derive(Default)]
pub(crate) struct ConfigurationReply {
    pending: Mutex<
        Option<(
            String,
            crate::ConfigurationProgressHandler,
            ConfigurationCompletion,
        )>,
    >,
    closed: AtomicBool,
}

impl ConfigurationReply {
    pub(crate) fn register(
        &self,
        id: String,
        progress: crate::ConfigurationProgressHandler,
        completion: ConfigurationCompletion,
    ) -> bool {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.closed.load(Ordering::Acquire) {
            drop(pending);
            completion(Err(SupervisorError::ChannelClosed));
            return false;
        }
        if pending.is_some() {
            drop(pending);
            completion(Err(SupervisorError::UnexpectedMessage(
                "configuration application is already pending".to_owned(),
            )));
            return false;
        }
        *pending = Some((id, progress, completion));
        true
    }

    pub(crate) fn fail(&self, error: SupervisorError) {
        self.closed.store(true, Ordering::Release);
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some((_, _, completion)) = pending {
            completion(Err(error));
        }
    }

    pub(crate) fn dispatch(&self, frame: &Result<Option<Message>, FrameError>) -> bool {
        if !matches!(frame, Ok(Some(_))) {
            self.closed.store(true, Ordering::Release);
        }
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some((id, progress_handler, _)) = pending.as_ref() else {
            return matches!(frame, Ok(Some(Message::ConfigurationProgress { .. })));
        };
        if let Ok(Some(Message::ConfigurationProgress {
            request_id,
            progress,
        })) = frame
        {
            if request_id != id {
                return true;
            }
            // Reject invalid progress but retain the lock until terminal acknowledgement.
            if let Err(error) = progress.validate() {
                tracing::warn!(%error, "invalid configuration progress");
                return true;
            }
            let handler = std::sync::Arc::clone(progress_handler);
            drop(pending);
            handler(progress.clone());
            return true;
        }
        let (result, consumed) = match frame {
            Ok(Some(Message::ConfigurationApplied { request_id })) if request_id == id => {
                (Ok(()), true)
            }
            Ok(Some(Message::Error {
                request_id: Some(request_id),
                code,
                message,
            })) if request_id == id => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "configuration failed: {code}: {message}"
                ))),
                true,
            ),
            Ok(None) => (Err(SupervisorError::ChannelClosed), false),
            Err(error) => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "configuration transport failed: {error}"
                ))),
                false,
            ),
            _ => return false,
        };
        let (_, _, completion) = pending.take().expect("pending configuration reply");
        drop(pending);
        completion(result);
        consumed
    }
}

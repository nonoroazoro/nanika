use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering}
};

use nanika_protocol::{FrameError, Message};

use crate::SupervisorError;

pub(crate) type ConfigurationCompletion = Box<dyn FnOnce(Result<(), SupervisorError>) + Send>;

/// One in-flight configuration reply, dispatched independently of search responses.
#[derive(Default)]
pub(crate) struct ConfigurationReply {
    pending: Mutex<Option<crate::PendingConfiguration>>,
    closed: AtomicBool
}

impl ConfigurationReply {
    pub(crate) fn register(
        &self,
        id: String,
        progress: crate::ConfigurationProgressHandler,
        resources: Option<std::sync::Arc<nanika_localization::ResourceSet>>,
        completion: ConfigurationCompletion
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
                "configuration application is already pending".to_owned()
            )));
            return false;
        }
        *pending = Some(crate::PendingConfiguration {
            id,
            progress,
            resources,
            completion
        });
        true
    }

    pub(crate) fn fail(&self, error: SupervisorError) {
        self.closed.store(true, Ordering::Release);
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(pending) = pending {
            (pending.completion)(Err(error));
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
        let Some(current) = pending.as_ref() else {
            return matches!(frame, Ok(Some(Message::ConfigurationProgress { .. })));
        };
        if let Ok(Some(Message::ConfigurationProgress {
            request_id,
            progress
        })) = frame
        {
            if request_id != &current.id {
                return true;
            }
            // Reject invalid progress but retain the accepted task until terminal acknowledgement.
            if let Err(error) = nanika_localization::validate_progress_presentation(
                progress,
                current.resources.as_deref()
            ) {
                tracing::warn!(request_id, %error, "invalid configuration progress");
                return true;
            }
            let handler = std::sync::Arc::clone(&current.progress);
            drop(pending);
            handler(progress.clone());
            return true;
        }
        let (result, consumed) = match frame {
            Ok(Some(Message::ConfigurationApplied { request_id })) if request_id == &current.id => {
                (Ok(()), true)
            }
            Ok(Some(Message::Error {
                request_id: Some(request_id),
                error
            })) if request_id == &current.id => (
                Err(crate::prepare_extension_error(
                    error.clone(),
                    current.resources.as_deref()
                )),
                true
            ),
            Ok(None) => (Err(SupervisorError::ChannelClosed), false),
            Err(error) => (
                Err(SupervisorError::UnexpectedMessage(format!(
                    "configuration transport failed: {error}"
                ))),
                false
            ),
            _ => return false
        };
        let completion = pending
            .take()
            .expect("pending configuration reply")
            .completion;
        drop(pending);
        completion(result);
        consumed
    }
}

#[cfg(test)]
#[path = "../tests/ConfigurationReply.rs"]
mod tests;

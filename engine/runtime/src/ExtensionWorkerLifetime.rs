use std::sync::{Arc, Condvar, Mutex};

use crate::{ExtensionNotifier, ExtensionSearchState};

/// Wake admission waiters even when initialization fails or the worker panics.
pub(crate) struct ExtensionWorkerLifetime {
    pub(crate) state: Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    pub(crate) notifier: ExtensionNotifier,
}

impl Drop for ExtensionWorkerLifetime {
    fn drop(&mut self) {
        let (invocations, view_events, configurations, refreshes) = {
            let (lock, changed) = &*self.state;
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.closed = true;
            let pending = (
                state.invocations.drain(..).collect::<Vec<_>>(),
                state.view_events.drain(..).collect::<Vec<_>>(),
                state.configurations.drain(..).collect::<Vec<_>>(),
                state.refreshes.drain(..).collect::<Vec<_>>(),
            );
            changed.notify_all();
            pending
        };
        for refresh in refreshes {
            let _ = refresh
                .completion
                .send(Err("extension worker closed before refreshing".to_owned()));
        }
        for invocation in invocations {
            let _ = invocation.response.send(Err(
                "extension worker closed before executing the action".to_owned(),
            ));
        }
        for request in view_events {
            let _ = request.completion.send(Err(
                "extension worker closed before handling the view request".to_owned(),
            ));
        }
        for update in configurations {
            update.complete(Err(
                "extension worker closed before applying the configuration".to_owned(),
            ));
        }
        let notify = self
            .notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(notify) = notify {
            notify();
        }
    }
}

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

use crate::{ExtensionConfigurationResult, ExtensionNotifier, ExtensionSearchState};

/// Wake admission waiters even when initialization fails or the worker panics.
pub(crate) struct ExtensionWorkerLifetime {
    pub(crate) extension_id: String,
    pub(crate) state: Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    pub(crate) configuration_results: Arc<Mutex<VecDeque<ExtensionConfigurationResult>>>,
    pub(crate) notifier: ExtensionNotifier,
}

impl Drop for ExtensionWorkerLifetime {
    fn drop(&mut self) {
        let (invocations, view_events, configurations) = {
            let (lock, changed) = &*self.state;
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.closed = true;
            let pending = (
                state.invocations.drain(..).collect::<Vec<_>>(),
                state.view_events.drain(..).collect::<Vec<_>>(),
                state.configurations.drain(..).collect::<Vec<_>>(),
            );
            changed.notify_all();
            pending
        };
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
        if !configurations.is_empty() {
            let mut results = self
                .configuration_results
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            results.extend(
                configurations
                    .into_iter()
                    .map(|update| ExtensionConfigurationResult {
                        extension_id: self.extension_id.clone(),
                        request_id: update.request_id,
                        result: Err(
                            "extension worker closed before applying the configuration".to_owned()
                        ),
                    }),
            );
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

use std::collections::HashSet;
use std::sync::{Arc, Condvar, Mutex};

use crate::ExtensionOperationReservation;

/// One accepted configuration or lifecycle transaction per installed extension.
#[derive(Default)]
pub(crate) struct ExtensionOperationGate {
    _state: Mutex<(bool, HashSet<String>)>,
    _idle: Condvar,
    _released: crate::ExtensionNotifier,
}

impl ExtensionOperationGate {
    pub(crate) fn reserve(
        self: &Arc<Self>,
        extension_id: &str,
    ) -> Result<ExtensionOperationReservation, String> {
        let mut state = self
            ._state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.0 {
            return Err("Extension operations are shutting down.".into());
        }
        if !state.1.insert(extension_id.to_owned()) {
            return Err(
                "A configuration or lifecycle operation is already pending for this extension."
                    .into(),
            );
        }
        Ok(ExtensionOperationReservation::new(
            Arc::clone(self),
            extension_id.to_owned(),
        ))
    }

    pub(crate) fn set_release_notifier(&self, notify: Arc<dyn Fn() + Send + Sync>) {
        *self
            ._released
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notify);
    }

    pub(crate) fn close(&self) {
        self._state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .0 = true;
    }

    pub(crate) fn wait_idle(&self) {
        let mut state = self
            ._state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while !state.1.is_empty() {
            state = self
                ._idle
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    pub(crate) fn release(&self, extension_id: &str) {
        self._state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .1
            .remove(extension_id);
        self._idle.notify_all();
        let notify = self
            ._released
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(notify) = notify {
            notify();
        }
    }
}

#[cfg(test)]
#[path = "../tests/ExtensionOperationGate.rs"]
mod tests;

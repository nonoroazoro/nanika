use nanika_extension_application::{DiscoveryWorker, RuntimeEvent};
use std::sync::mpsc::{Receiver, RecvError};

/// Own the bounded consumer before its producer so every error path releases
/// blocked event sends before joining the discovery thread.
pub(crate) struct DiscoveryRuntime {
    _events: Receiver<RuntimeEvent>,
    pub(crate) worker: DiscoveryWorker,
}

impl DiscoveryRuntime {
    pub(crate) fn new(events: Receiver<RuntimeEvent>, worker: DiscoveryWorker) -> Self {
        Self {
            _events: events,
            worker,
        }
    }

    pub(crate) fn receive(&self) -> Result<RuntimeEvent, RecvError> {
        self._events.recv()
    }

    pub(crate) fn shutdown(self) -> Result<(), String> {
        self.worker.shutdown(self._events)
    }
}

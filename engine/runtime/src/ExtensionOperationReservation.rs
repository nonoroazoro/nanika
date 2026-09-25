use crate::ExtensionOperationGate;
use std::sync::Arc;

pub(crate) struct ExtensionOperationReservation {
    _gate: Arc<ExtensionOperationGate>,
    _extension_id: String,
}

impl ExtensionOperationReservation {
    pub(crate) fn new(gate: Arc<ExtensionOperationGate>, extension_id: String) -> Self {
        Self {
            _gate: gate,
            _extension_id: extension_id,
        }
    }
}

impl Drop for ExtensionOperationReservation {
    fn drop(&mut self) {
        self._gate.release(&self._extension_id);
    }
}

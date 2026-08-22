use nanika_storage::ExtensionKind;

use crate::ExtensionProcess;

/// Spawned extension waiting for registration after host runtime initialization.
pub(crate) struct PendingExtension {
    pub(crate) extension_id: String,
    pub(crate) kind: ExtensionKind,
    pub(crate) process: ExtensionProcess,
}

use nanika_extension_package::ExtensionContributions;
use nanika_storage::ExtensionKind;

use crate::ExtensionRuntime;

/// Spawned extension waiting for registration after host runtime initialization.
pub(crate) struct PendingExtension {
    pub(crate) extension_id: String,
    pub(crate) kind: ExtensionKind,
    pub(crate) permissions: Vec<String>,
    pub(crate) contributions: ExtensionContributions,
    pub(crate) runtime: ExtensionRuntime,
}

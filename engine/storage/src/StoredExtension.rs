use std::path::PathBuf;

use crate::ExtensionKind;

/// Machine-local extension installation state loaded by the storage owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredExtension {
    pub extension_id: String,
    pub kind: ExtensionKind,
    pub installed_version: Option<String>,
    pub active_version: Option<String>,
    pub install_path: Option<PathBuf>,
    pub package_digest: Option<String>,
    pub state: String,
    pub health: String,
    pub last_error: Option<String>,
}

use std::path::PathBuf;

use crate::ExtensionProtocol;

/// Validated external extension ready for host-supervised process creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveExtension {
    pub extension_id: String,
    pub program: PathBuf,
    pub protocol: ExtensionProtocol,
    pub permissions: Vec<String>,
}

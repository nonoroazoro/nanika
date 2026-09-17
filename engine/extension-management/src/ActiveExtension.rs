use std::path::PathBuf;

use crate::{ExtensionContributions, ExtensionManifest, ExtensionProtocol};

/// Validated extension ready for host-supervised process creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveExtension {
    pub extension_id: String,
    pub program: PathBuf,
    pub protocol: ExtensionProtocol,
    pub permissions: Vec<String>,
    pub contributes: ExtensionContributions,
}

impl ActiveExtension {
    /// Convert one validated manifest into the common runtime input.
    pub fn from_manifest(manifest: ExtensionManifest, program: PathBuf) -> Self {
        Self {
            extension_id: manifest.id,
            program,
            protocol: manifest.runtime,
            permissions: manifest.permissions,
            contributes: manifest.contributes,
        }
    }
}

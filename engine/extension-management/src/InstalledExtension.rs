use std::path::PathBuf;

use crate::{ExtensionContributions, ExtensionManifest, ExtensionProtocol};

/// Validated installed descriptor, independent of enablement or a live process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledExtension {
    pub extension_id: String,
    pub name: String,
    pub icon: String,
    pub program: PathBuf,
    pub resource_root: PathBuf,
    pub protocol: ExtensionProtocol,
    pub activation: crate::ExtensionActivation,
    pub permissions: Vec<String>,
    pub contributes: ExtensionContributions,
}

impl InstalledExtension {
    /// Convert one validated manifest into the common runtime input.
    pub fn from_manifest(
        manifest: ExtensionManifest,
        program: PathBuf,
        resource_root: PathBuf,
    ) -> Self {
        Self {
            extension_id: manifest.id,
            name: manifest.name,
            icon: manifest.icon,
            program,
            resource_root,
            protocol: manifest.runtime,
            activation: manifest.activation,
            permissions: manifest.permissions,
            contributes: manifest.contributes,
        }
    }
}

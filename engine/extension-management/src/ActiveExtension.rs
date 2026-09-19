use std::path::PathBuf;

use crate::{ExtensionContributions, ExtensionManifest, ExtensionProtocol};

/// Validated extension ready for host-supervised process creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveExtension {
    pub extension_id: String,
    pub name: String,
    pub icon: crate::ContributionIcon,
    pub program: PathBuf,
    pub protocol: ExtensionProtocol,
    pub activation: crate::ExtensionActivation,
    pub permissions: Vec<String>,
    pub contributes: ExtensionContributions,
}

impl ActiveExtension {
    /// Convert one validated manifest into the common runtime input.
    pub fn from_manifest(manifest: ExtensionManifest, program: PathBuf) -> Self {
        Self {
            extension_id: manifest.id,
            name: manifest.name,
            icon: manifest.icon,
            program,
            protocol: manifest.runtime,
            activation: manifest.activation,
            permissions: manifest.permissions,
            contributes: manifest.contributes,
        }
    }
}

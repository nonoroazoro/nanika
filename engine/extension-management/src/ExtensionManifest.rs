use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ExtensionContributions, ExtensionProtocol, ExtensionTarget};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionManifest {
    pub format: String,
    pub manifest_version: u32,
    pub id: String,
    pub name: nanika_protocol::LocalizedText,
    pub icon: String,
    pub version: String,
    pub host_api: String,
    pub targets: BTreeMap<String, ExtensionTarget>,
    pub runtime: ExtensionProtocol,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub localization: Option<nanika_localization::LocalizationResources>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub activation: crate::ExtensionActivation,
    #[serde(default)]
    pub contributes: ExtensionContributions
}

pub(crate) fn _validate_name(value: &str) -> Result<(), crate::ExtensionPackageError> {
    if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(crate::ExtensionPackageError::Manifest(
            "extension name must contain 1 to 128 UTF-8 bytes without control characters".into()
        ));
    }
    Ok(())
}

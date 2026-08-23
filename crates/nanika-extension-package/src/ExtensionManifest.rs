use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ExtensionTarget;

/// Typed root manifest for one external extension version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionManifest {
    pub format: String,
    pub manifest_version: u32,
    pub id: String,
    pub version: String,
    pub host_api: String,
    pub targets: BTreeMap<String, ExtensionTarget>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub activation_events: Vec<String>,
    #[serde(default)]
    pub contributions: serde_json::Value,
}

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Machine-local locator for the relocatable synchronized configuration tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapConfig {
    pub format_version: u32,
    pub config_root: PathBuf,
    pub machine_id: Uuid,
}

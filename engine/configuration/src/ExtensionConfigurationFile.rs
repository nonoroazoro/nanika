use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CONFIG_FORMAT_VERSION;

/// Host-owned persisted values for one extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionConfigurationFile {
    pub format_version: u32,
    pub values: BTreeMap<String, Value>,
}

impl ExtensionConfigurationFile {
    pub fn new(values: BTreeMap<String, Value>) -> Self {
        Self {
            format_version: CONFIG_FORMAT_VERSION,
            values,
        }
    }

    pub fn validate_format(&self) -> Result<(), String> {
        if self.format_version == CONFIG_FORMAT_VERSION {
            Ok(())
        } else {
            Err(format!(
                "unsupported extension configuration format {}",
                self.format_version
            ))
        }
    }
}

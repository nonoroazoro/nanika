use serde::{Deserialize, Serialize};

/// Target-specific native entrypoint declared by an extension package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionTarget {
    pub entrypoint: String,
}

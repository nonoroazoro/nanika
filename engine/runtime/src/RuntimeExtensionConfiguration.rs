use std::collections::BTreeMap;

use nanika_extension_package::ConfigurationContribution;
use serde_json::Value;

/// Static schema and current effective values exposed to the future Settings UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExtensionConfiguration {
    pub extension_id: String,
    pub contribution: ConfigurationContribution,
    pub values: BTreeMap<String, Value>,
}

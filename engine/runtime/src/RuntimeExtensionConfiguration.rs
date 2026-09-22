use std::collections::BTreeMap;

use nanika_extension_package::ConfigurationContribution;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExtensionConfiguration {
    pub extension_id: String,
    pub contribution: ConfigurationContribution,
    pub values: BTreeMap<String, Value>,
}

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ConfigurationSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationProperty {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub default: Value,
    #[serde(flatten)]
    pub schema: ConfigurationSchema,
}

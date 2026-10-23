use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationMenuGroup {
    pub id: String,
    pub title: nanika_protocol::LocalizedText,
    pub items: Vec<crate::ConfigurationMenuItem>
}

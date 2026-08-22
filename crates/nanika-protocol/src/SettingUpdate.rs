use serde::{Deserialize, Serialize};

use crate::SettingValue;

/// One edited setting returned to its owning extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingUpdate {
    pub key: String,
    pub value: SettingValue,
}

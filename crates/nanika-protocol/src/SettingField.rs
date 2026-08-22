use serde::{Deserialize, Serialize};

use crate::{SettingControl, SettingValue};

/// One extension-owned setting exposed through the common host view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingField {
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub control: SettingControl,
    pub value: SettingValue,
}

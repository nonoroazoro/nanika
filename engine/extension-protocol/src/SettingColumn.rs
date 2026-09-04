use serde::{Deserialize, Serialize};

use crate::SettingColumnControl;

/// One column in a generic record-table setting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingColumn {
    pub key: String,
    pub title: String,
    pub control: SettingColumnControl,
    pub required: bool,
}

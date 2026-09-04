use serde::{Deserialize, Serialize};

/// Editor used by one record-table column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SettingColumnControl {
    Text {
        placeholder: Option<String>,
        path: bool,
    },
    StringList {
        placeholder: Option<String>,
        max_items: u32,
    },
}

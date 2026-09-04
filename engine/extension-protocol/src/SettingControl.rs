use serde::{Deserialize, Serialize};

use crate::SettingColumn;

/// Declarative control rendered by the host without domain knowledge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SettingControl {
    Toggle,
    Text {
        placeholder: Option<String>,
        path: bool,
    },
    StringList {
        placeholder: Option<String>,
        path: bool,
        max_items: u32,
    },
    RecordTable {
        columns: Vec<SettingColumn>,
        max_rows: u32,
    },
}

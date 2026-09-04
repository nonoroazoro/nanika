use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One typed value edited by the shared Settings UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SettingValue {
    Boolean {
        value: bool,
    },
    Text {
        value: String,
    },
    StringList {
        values: Vec<String>,
    },
    Records {
        rows: Vec<BTreeMap<String, SettingValue>>,
    },
}

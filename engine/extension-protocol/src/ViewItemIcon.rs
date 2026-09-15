use serde::{Deserialize, Serialize};

/// Product-owned semantic icon rendered for a declarative list item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ViewItemIcon {
    Text,
    Files,
    Image,
}

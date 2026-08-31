use serde::{Deserialize, Serialize};

/// Host-styled prominence for an extension view action.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ViewActionStyle {
    Primary,
    Secondary,
    Destructive,
}

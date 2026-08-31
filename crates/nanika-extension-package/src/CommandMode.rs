use serde::{Deserialize, Serialize};

/// Presentation behavior for a command contributed to Root Search.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommandMode {
    NoView,
    View,
}

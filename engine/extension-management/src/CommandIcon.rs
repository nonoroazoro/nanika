use serde::{Deserialize, Serialize};

/// A bounded host-rendered icon for a statically contributed command.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommandIcon {
    Clipboard,
}

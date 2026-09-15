use serde::{Deserialize, Serialize};

/// A bounded command identity icon rendered entirely by the host.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommandIcon {
    Clipboard,
}

impl CommandIcon {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
        }
    }
}

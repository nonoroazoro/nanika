use serde::{Deserialize, Serialize};

/// A bounded static-contribution icon rendered entirely by the host.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContributionIcon {
    Applications,
    Calculator,
    Clipboard,
    Command,
    Script,
    Extension,
}

impl ContributionIcon {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applications => "applications",
            Self::Calculator => "calculator",
            Self::Clipboard => "clipboard",
            Self::Command => "command",
            Self::Script => "script",
            Self::Extension => "extension",
        }
    }
}

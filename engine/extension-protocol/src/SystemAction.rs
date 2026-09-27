use serde::{Deserialize, Serialize};

/// Closed set of native operations. Arbitrary commands never cross this boundary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SystemAction {
    Lock,
    Sleep,
    TurnOffDisplays,
    LogOut,
    Restart,
    ShutDown,
    OpenTrash,
    EmptyTrash,
}

impl SystemAction {
    pub const fn permission(self) -> &'static str {
        match self {
            Self::Lock => "system.lock",
            Self::Sleep => "system.sleep",
            Self::TurnOffDisplays => "system.displays",
            Self::LogOut => "system.logout",
            Self::Restart => "system.restart",
            Self::ShutDown => "system.shutdown",
            Self::OpenTrash => "system.trash.open",
            Self::EmptyTrash => "system.trash.empty",
        }
    }
}

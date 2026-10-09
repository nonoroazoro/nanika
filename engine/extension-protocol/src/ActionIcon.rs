use serde::{Deserialize, Serialize};

/// Host-rendered symbols. Extensions select identities, never markup or remote URLs.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ActionIcon {
    FolderOpen,
    LockKeyhole,
    LogOut,
    MonitorOff,
    Moon,
    Power,
    RotateCw,
    Trash
}

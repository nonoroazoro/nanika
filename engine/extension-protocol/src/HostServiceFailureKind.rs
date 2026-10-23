use serde::{Deserialize, Serialize};

/// The admission and completion facts of a host service request.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HostServiceFailureKind {
    /// Cancelled before the native service accepted responsibility.
    Cancelled,
    /// Preparation or submission failed without admitting a native effect.
    Rejected,
    /// The accepted service returned an explicit failure.
    Failed,
    /// The accepted service closed without reporting its outcome.
    Unconfirmed
}

impl HostServiceFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
            Self::Unconfirmed => "unconfirmed"
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Rejected => "host_service_rejected",
            Self::Failed => "host_service_failed",
            Self::Unconfirmed => "host_service_unconfirmed"
        }
    }
}

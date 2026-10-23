use crate::{HostServiceFailureKind, ProtocolError, message};
use serde::{Deserialize, Serialize};

/// Host-owned outcome facts and original diagnostics, without a resource namespace.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostServiceFailure {
    pub kind: HostServiceFailureKind,
    pub detail: String
}

impl HostServiceFailure {
    pub fn new(kind: HostServiceFailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into()
        }
    }

    /// Report this outcome using an extension-owned message with an `outcome` selector.
    /// The host binds the reference to the sender's resources when receiving the error.
    pub fn into_error(self, message_id: impl Into<String>) -> ProtocolError {
        ProtocolError::new(
            self.kind.code(),
            message(message_id, [("outcome".into(), self.kind.as_str().into())])
        )
        .with_detail(self.detail)
    }
}

impl std::fmt::Debug for HostServiceFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostServiceFailure")
            .field("kind", &self.kind)
            .field("detail", &"<redacted>")
            .finish()
    }
}

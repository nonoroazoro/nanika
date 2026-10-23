use crate::{OperationErrorCode, SupervisorError};

/// An operation's retained presentation, independent of process and window lifetimes.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OperationError {
    Host {
        code: OperationErrorCode,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        detail: Option<String>
    },
    Extension {
        error: Box<nanika_localization::PreparedError>
    },
    /// One failed navigation and the failed attempt to release its new view.
    /// Both children are ordinary host/extension failures, never nested cleanup groups.
    Cleanup {
        primary: Box<OperationError>,
        cleanup: Box<OperationError>
    }
}

impl OperationError {
    pub fn new(code: OperationErrorCode) -> Self {
        Self::Host { code, detail: None }
    }

    pub fn with_detail(code: OperationErrorCode, detail: impl Into<String>) -> Self {
        Self::Host {
            code,
            detail: Some(detail.into())
        }
    }
}

impl From<&SupervisorError> for OperationError {
    fn from(error: &SupervisorError) -> Self {
        let code = match error {
            SupervisorError::Reported(error) => {
                return Self::Extension {
                    error: error.clone()
                };
            }
            SupervisorError::ChannelClosed => OperationErrorCode::ExtensionUnavailable,
            SupervisorError::Cancelled(_) => OperationErrorCode::Cancelled,
            SupervisorError::Terminated(_) => OperationErrorCode::ExtensionTerminated,
            SupervisorError::Io(_) => OperationErrorCode::ExtensionIo,
            SupervisorError::Protocol(_) | SupervisorError::UnexpectedMessage(_) => {
                OperationErrorCode::ExtensionProtocol
            }
        };
        Self::with_detail(code, error.to_string())
    }
}

impl From<SupervisorError> for OperationError {
    fn from(error: SupervisorError) -> Self {
        Self::from(&error)
    }
}

impl std::fmt::Debug for OperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host { code, detail } => formatter
                .debug_struct("OperationError")
                .field("code", code)
                .field("detail", &detail.as_ref().map(|_| "<redacted>"))
                .finish(),
            Self::Extension { error } => formatter
                .debug_tuple("OperationError")
                .field(error)
                .finish(),
            Self::Cleanup { primary, cleanup } => formatter
                .debug_struct("OperationError::Cleanup")
                .field("primary", primary)
                .field("cleanup", cleanup)
                .finish()
        }
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host { code, detail } => {
                write!(formatter, "{code:?}")?;
                if let Some(detail) = detail {
                    write!(formatter, ": {detail}")?;
                }
                Ok(())
            }
            Self::Extension { error } => write!(formatter, "{error}"),
            Self::Cleanup { primary, cleanup } => {
                write!(formatter, "{primary}; cleanup failed: {cleanup}")
            }
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
#[path = "../tests/OperationError.rs"]
mod tests;

use serde::Serialize;

/// Transaction facts remain machine-readable independently of presentation language.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum ConfigurationSaveError {
    Rejected { cause: Box<crate::OperationError> },
    Unconfirmed { cause: Box<crate::OperationError> },
    SaveFailed { detail: String },
    AppliedButNotSaved { detail: String },
    Unavailable,
    UnconfirmedNotSaved { cause: Box<crate::OperationError> },
    SavedButNotApplied { cause: Box<crate::OperationError> }
}

impl ConfigurationSaveError {
    /// The operation did not enter the worker or change persistence/application state.
    pub fn rejected(cause: crate::OperationError) -> Self {
        Self::Rejected {
            cause: Box::new(cause)
        }
    }

    /// A lost completion cannot establish whether the accepted change took effect.
    pub fn unconfirmed(cause: crate::OperationError) -> Self {
        Self::Unconfirmed {
            cause: Box::new(cause)
        }
    }
}

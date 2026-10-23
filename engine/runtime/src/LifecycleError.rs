use crate::LifecycleFailure;

/// Recovery presentation retains causes without translating or recursively wrapping them.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum LifecycleError {
    Failed {
        failure: Box<LifecycleFailure>
    },
    Restarting {
        failure: Box<LifecycleFailure>
    },
    Stopping {
        failure: Box<LifecycleFailure>
    },
    RestartFailed {
        original: Box<LifecycleFailure>,
        failure: Box<LifecycleFailure>
    },
    RestartLimitReached {
        original: Box<LifecycleFailure>,
        failure: Box<LifecycleFailure>
    },
    RecoveryStopped {
        original: Box<LifecycleFailure>,
        failures: Vec<LifecycleFailure>
    }
}

impl From<LifecycleFailure> for LifecycleError {
    fn from(failure: LifecycleFailure) -> Self {
        Self::Failed {
            failure: Box::new(failure)
        }
    }
}

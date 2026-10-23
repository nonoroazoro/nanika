use crate::{OperationError, OperationErrorCode, SupervisorError};

/// Accommodates the 64 KiB process tail after lossy UTF-8 decoding and context.
pub const MAX_LIFECYCLE_DIAGNOSTIC_BYTES: usize = 256 * 1024;

/// Retains the admitted error independently of supplemental process diagnostics.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, serde::Serialize, PartialEq, Eq)]
pub struct LifecycleFailure {
    pub error: OperationError,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub diagnostic: Option<String>
}

impl LifecycleFailure {
    pub fn new(code: OperationErrorCode) -> Self {
        OperationError::new(code).into()
    }

    pub fn with_diagnostic(mut self, diagnostic: Option<String>) -> Self {
        self.diagnostic = diagnostic.map(|mut text| {
            if text.len() > MAX_LIFECYCLE_DIAGNOSTIC_BYTES {
                const OMITTED: &str = "[earlier diagnostic omitted]\n";
                let mut start = text.len() - (MAX_LIFECYCLE_DIAGNOSTIC_BYTES - OMITTED.len());
                while !text.is_char_boundary(start) {
                    start += 1;
                }
                text.replace_range(..start, OMITTED);
            }
            text
        });
        self
    }
}

impl From<OperationError> for LifecycleFailure {
    fn from(error: OperationError) -> Self {
        Self {
            error,
            diagnostic: None
        }
    }
}

impl From<SupervisorError> for LifecycleFailure {
    fn from(error: SupervisorError) -> Self {
        OperationError::from(error).into()
    }
}

impl std::fmt::Debug for LifecycleFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LifecycleFailure")
            .field("error", &self.error)
            .field(
                "diagnostic",
                &self.diagnostic.as_ref().map(|_| "<redacted>")
            )
            .finish()
    }
}

impl std::fmt::Display for LifecycleFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.error)?;
        if let Some(diagnostic) = &self.diagnostic {
            write!(formatter, "\n{diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LifecycleFailure {}

#[cfg(test)]
#[path = "../tests/LifecycleFailure.rs"]
mod tests;

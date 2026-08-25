/// One external extension resolution failure with a redaction-safe diagnostic context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionResolutionError {
    pub diagnostic_context: String,
    pub message: String,
}

impl ExtensionResolutionError {
    pub(crate) fn new(diagnostic_context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            diagnostic_context: diagnostic_context.into(),
            message: message.into(),
        }
    }
}

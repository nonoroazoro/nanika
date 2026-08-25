use nanika_extension_package::ExtensionResolutionError;

/// One extension startup failure with a redaction-safe diagnostic context.
pub(crate) struct ExtensionStartupError {
    pub(crate) diagnostic_context: String,
    pub(crate) source: String,
}

impl ExtensionStartupError {
    pub(crate) fn new(diagnostic_context: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            diagnostic_context: diagnostic_context.into(),
            source: source.into(),
        }
    }
}

impl From<ExtensionResolutionError> for ExtensionStartupError {
    fn from(error: ExtensionResolutionError) -> Self {
        Self::new(error.diagnostic_context, error.message)
    }
}

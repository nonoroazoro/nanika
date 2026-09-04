use crate::DiagnosticCode;

/// Bounded duplicate-suppression identity for one operational event.
#[derive(Eq, Hash, PartialEq)]
pub(crate) struct DiagnosticRecordKey {
    code: DiagnosticCode,
    operation: &'static str,
    safe_context: Option<String>,
    error: bool,
}

impl DiagnosticRecordKey {
    pub(crate) fn new(
        code: DiagnosticCode,
        operation: &'static str,
        safe_context: Option<String>,
        error: bool,
    ) -> Self {
        Self {
            code,
            operation,
            safe_context,
            error,
        }
    }
}

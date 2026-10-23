use crate::SearchSession;
use nanika_host::{LifecycleFailure, OperationError, OperationErrorCode as Code};

#[derive(Default)]
pub(crate) struct DesktopRuntime {
    pub(crate) runtime: Option<std::sync::Arc<nanika_host::RuntimeService>>,
    pub(crate) localization: nanika_localization::LocalizationSnapshot,
    pub(crate) startup_error: Option<LifecycleFailure>,
    pub(crate) session: Option<SearchSession>
}

impl DesktopRuntime {
    pub(crate) fn fail_startup(&mut self, error: OperationError) {
        let (code, diagnostic) = match error {
            OperationError::Host { code, detail } => (code, detail),
            error => (Code::RuntimeStartupFailed, Some(error.to_string()))
        };
        // Channel updates expose only the cause. Internal diagnostics require an explicit read.
        self.startup_error = Some(LifecycleFailure::new(code).with_diagnostic(diagnostic));
    }

    pub(crate) fn startup_diagnostic(
        &self,
        session_id: u64
    ) -> Result<LifecycleFailure, OperationError> {
        self.session
            .as_ref()
            .ok_or_else(|| OperationError::new(Code::SessionUnavailable))?
            .authorize(session_id)?;
        self.startup_error
            .clone()
            .ok_or_else(|| OperationError::new(Code::StartupDiagnosticUnavailable))
    }
}

#[cfg(test)]
#[path = "../tests/DesktopRuntime.rs"]
mod tests;

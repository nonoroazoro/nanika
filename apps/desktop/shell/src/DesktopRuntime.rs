use crate::SearchSession;

#[derive(Default)]
pub(crate) struct DesktopRuntime {
    pub(crate) runtime: Option<std::sync::Arc<nanika_host::RuntimeService>>,
    pub(crate) startup_error: Option<String>,
    pub(crate) session: Option<SearchSession>,
}

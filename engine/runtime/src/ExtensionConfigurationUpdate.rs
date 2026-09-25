use nanika_protocol::ExtensionConfiguration;

/// One accepted configuration operation waiting for an extension worker.
pub(crate) struct ExtensionConfigurationUpdate {
    pub(crate) request_id: String,
    pub(crate) configuration: ExtensionConfiguration,
    pub(crate) require_live: bool,
    pub(crate) progress: crate::ConfigurationProgressHandler,
    pub(crate) completion:
        std::sync::mpsc::SyncSender<Result<crate::ConfigurationApplication, String>>,
}

impl ExtensionConfigurationUpdate {
    pub(crate) fn complete(self, result: Result<crate::ConfigurationApplication, String>) {
        let _ = self.completion.send(result);
    }
}

impl std::fmt::Debug for ExtensionConfigurationUpdate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExtensionConfigurationUpdate")
            .field("request_id", &self.request_id)
            .finish_non_exhaustive()
    }
}

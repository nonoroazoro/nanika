use nanika_protocol::SettingsContribution;

/// One settings response produced by an extension worker.
#[derive(Debug)]
pub(crate) struct ExtensionSettingsResult {
    pub(crate) extension_id: String,
    /// `None` identifies the initial contribution; updates retain their request identity.
    pub(crate) request_id: Option<String>,
    pub(crate) result: Result<SettingsContribution, String>,
}

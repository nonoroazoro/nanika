use nanika_protocol::SettingsContribution;

/// Settings contribution delivered across the runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSettingsUpdate {
    pub extension_id: String,
    pub request_id: Option<String>,
    pub result: Result<SettingsContribution, String>,
}

/// Application result produced by a running Nanika extension.
#[derive(Debug)]
pub(crate) struct ExtensionConfigurationResult {
    pub(crate) extension_id: String,
    pub(crate) request_id: String,
    pub(crate) result: Result<(), String>,
}

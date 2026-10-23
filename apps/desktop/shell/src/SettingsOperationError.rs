/// Command rejection preserves configuration facts for the Launcher as well as Settings.
#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum SettingsOperationError {
    Configuration(nanika_host::ConfigurationSaveError),
    Operation(nanika_host::OperationError)
}

impl From<nanika_host::ConfigurationSaveError> for SettingsOperationError {
    fn from(error: nanika_host::ConfigurationSaveError) -> Self {
        Self::Configuration(error)
    }
}

impl From<nanika_host::OperationError> for SettingsOperationError {
    fn from(error: nanika_host::OperationError) -> Self {
        Self::Operation(error)
    }
}

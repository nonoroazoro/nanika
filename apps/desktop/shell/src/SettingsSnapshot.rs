use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSnapshot {
    pub(crate) lifecycle_revision: u64,
    pub(crate) maximized: bool,
    pub(crate) version: &'static str,
    pub(crate) general: nanika_config::LauncherPreferences,
    pub(crate) extensions: Vec<ExtensionSettings>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionSettings {
    #[serde(flatten)]
    pub(crate) info: nanika_host::RuntimeExtensionInfo,
    pub(crate) configuration: Option<nanika_host::RuntimeExtensionConfiguration>,
    pub(crate) application: Option<SettingsApplicationUpdate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveSettingsRequest {
    pub(crate) extension_id: String,
    pub(crate) key: String,
    pub(crate) value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum SettingsSaveResult {
    Running {
        progress: Option<nanika_protocol::OperationProgress>,
    },
    Completed {
        #[serde(flatten)]
        outcome: nanika_host::ConfigurationSaveOutcome,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsApplicationUpdate {
    pub(crate) request_id: u64,
    pub(crate) extension_id: String,
    pub(crate) key: String,
    pub(crate) result: SettingsSaveResult,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum SettingsEvent {
    Lifecycle {
        #[serde(rename = "deliveryId")]
        delivery_id: u64,
        revision: u64,
        extensions: Vec<crate::ExtensionLifecycle>,
    },
    Application {
        update: SettingsApplicationUpdate,
        #[serde(rename = "deliveryId", skip_serializing_if = "Option::is_none")]
        delivery_id: Option<u64>,
    },
    Closed,
    WindowState {
        maximized: bool,
    },
    ShortcutPressed,
}

impl From<Result<nanika_host::ConfigurationSaveOutcome, String>> for SettingsSaveResult {
    fn from(result: Result<nanika_host::ConfigurationSaveOutcome, String>) -> Self {
        match result {
            Ok(outcome) => Self::Completed { outcome },
            Err(error) => Self::Failed { error },
        }
    }
}

pub(crate) fn validate_settings_request(request: &SaveSettingsRequest) -> Result<(), String> {
    if !nanika_foundation::is_valid_extension_id(&request.extension_id)
        || request.key.is_empty()
        || request.key.len() > 128
    {
        return Err("Invalid extension settings request.".to_owned());
    }
    Ok(())
}

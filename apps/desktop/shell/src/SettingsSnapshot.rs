use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSnapshot {
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
    pub(crate) configuration: nanika_host::RuntimeExtensionConfiguration,
    pub(crate) application: Option<SettingsApplicationUpdate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveSettingsRequest {
    pub(crate) extension_id: String,
    pub(crate) values: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSaveResult {
    pub(crate) status: SettingsSaveStatus,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SettingsSaveStatus {
    Applying,
    Applied,
    NextLaunch,
    ApplyFailed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsApplicationUpdate {
    pub(crate) request_id: u64,
    pub(crate) extension_id: String,
    pub(crate) result: SettingsSaveResult,
}

#[derive(Default)]
pub(crate) struct SettingsApplications {
    pub(crate) latest: BTreeMap<String, SettingsApplicationUpdate>,
    pub(crate) updates: Option<tauri::ipc::Channel<SettingsEvent>>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum SettingsEvent {
    Application { update: SettingsApplicationUpdate },
    Closed,
    WindowState { maximized: bool },
    ShortcutPressed,
}

impl From<nanika_host::ConfigurationSaveOutcome> for SettingsSaveResult {
    fn from(outcome: nanika_host::ConfigurationSaveOutcome) -> Self {
        use nanika_host::ConfigurationSaveOutcome;
        let (status, error) = match outcome {
            ConfigurationSaveOutcome::Applied => (SettingsSaveStatus::Applied, None),
            ConfigurationSaveOutcome::SavedForNextLaunch => (SettingsSaveStatus::NextLaunch, None),
            ConfigurationSaveOutcome::ApplyFailed(error) => {
                (SettingsSaveStatus::ApplyFailed, Some(error))
            }
        };
        Self { status, error }
    }
}

pub(crate) fn validate_settings_request(request: &SaveSettingsRequest) -> Result<(), String> {
    if !nanika_foundation::is_valid_extension_id(&request.extension_id) || request.values.len() > 64
    {
        return Err("Invalid extension settings request.".to_owned());
    }
    let bytes = serde_json::to_vec(&request.values).map_err(|error| error.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("Extension settings exceed 1 MiB.".to_owned());
    }
    Ok(())
}

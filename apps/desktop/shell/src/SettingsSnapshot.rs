use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSnapshot {
    pub(crate) session_id: u64,
    pub(crate) platform: nanika_platform::TargetPlatform,
    pub(crate) localization: nanika_localization::LocalizationSnapshot,
    pub(crate) navigation: crate::SettingsNavigation,
    pub(crate) lifecycle_revision: u64,
    pub(crate) maximized: bool,
    pub(crate) version: &'static str,
    pub(crate) general: nanika_config::LauncherPreferences,
    pub(crate) general_sections: Vec<crate::GeneralSettingsSection>,
    pub(crate) extensions: Vec<ExtensionSettings>
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionSettings {
    #[serde(flatten)]
    pub(crate) info: nanika_host::RuntimeExtensionInfo,
    pub(crate) icon_url: String,
    pub(crate) configuration: Option<nanika_host::RuntimeExtensionConfiguration>,
    pub(crate) application: Option<SettingsApplicationUpdate>
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveSettingsRequest {
    pub(crate) extension_id: String,
    pub(crate) key: String,
    pub(crate) value: serde_json::Value
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum SettingsSaveResult {
    Running {
        progress: Option<nanika_protocol::OperationProgress>
    },
    Completed {
        #[serde(flatten)]
        outcome: nanika_host::ConfigurationSaveOutcome
    },
    Failed {
        error: nanika_host::ConfigurationSaveError
    }
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsApplicationUpdate {
    pub(crate) request_id: u64,
    pub(crate) extension_id: String,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resource_scope: Option<nanika_host::ExtensionResourceScope>,
    pub(crate) key: String,
    pub(crate) result: SettingsSaveResult
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum SettingsEvent {
    Localization {
        #[serde(rename = "sessionId")]
        session_id: u64,
        #[serde(rename = "deliveryId")]
        delivery_id: u64,
        localization: nanika_localization::LocalizationSnapshot
    },
    Navigation {
        navigation: crate::SettingsNavigation
    },
    Lifecycle {
        #[serde(rename = "deliveryId")]
        delivery_id: u64,
        revision: u64,
        extensions: Vec<crate::ExtensionLifecycle>
    },
    Application {
        update: SettingsApplicationUpdate,
        #[cfg_attr(feature = "typescript", ts(optional))]
        #[serde(rename = "deliveryId", skip_serializing_if = "Option::is_none")]
        delivery_id: Option<u64>
    },
    Closed,
    WindowState {
        maximized: bool
    },
    ShortcutPressed
}

impl From<Result<nanika_host::ConfigurationSaveOutcome, nanika_host::OperationError>>
    for SettingsSaveResult
{
    fn from(
        result: Result<nanika_host::ConfigurationSaveOutcome, nanika_host::OperationError>
    ) -> Self {
        match result {
            Ok(outcome) => Self::Completed { outcome },
            Err(error) => Self::Failed {
                error: nanika_host::ConfigurationSaveError::unconfirmed(error)
            }
        }
    }
}

pub(crate) fn validate_settings_request(
    request: &SaveSettingsRequest
) -> Result<(), nanika_host::OperationError> {
    if !nanika_foundation::is_valid_extension_id(&request.extension_id)
        || request.key.is_empty()
        || request.key.len() > 128
    {
        return Err(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::InvalidConfiguration
        ));
    }
    Ok(())
}

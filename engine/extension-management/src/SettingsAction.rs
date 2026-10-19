use serde::{Deserialize, Serialize};

/// An explicit operation exposed in Settings, never persisted as a configuration value.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsAction {
    pub action: nanika_protocol::Action,
    pub description: String,
    /// Concise button text when the adjacent Settings title already names the operation.
    #[serde(default)]
    pub button_label: Option<String>
}

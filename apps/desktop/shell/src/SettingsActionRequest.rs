#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SettingsActionRequest {
    pub(crate) extension_id: String,
    pub(crate) instance_id: u64,
    pub(crate) action_id: String,
    pub(crate) confirmed: bool
}

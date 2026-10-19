/// Latest requested Settings destination, retained until its WebView is ready.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsNavigation {
    pub(crate) revision: u64,
    pub(crate) extension_id: Option<String>
}

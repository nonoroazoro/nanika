#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSearchEntry {
    pub(crate) page_id: String,
    pub(crate) target: crate::SettingsSearchTarget,
    pub(crate) title: nanika_protocol::LocalizedText,
    pub(crate) page_title: nanika_protocol::LocalizedText,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title_resource_scope: Option<nanika_host::ExtensionResourceScope>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) page_resource_scope: Option<nanika_host::ExtensionResourceScope>,
    #[serde(skip)]
    pub(crate) search_values: Vec<String>
}

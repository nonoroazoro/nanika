use serde::Serialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionViewSnapshot {
    pub(crate) route_id: u64,
    pub(crate) extension_id: String,
    pub(crate) extension_name: nanika_protocol::LocalizedText,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resource_scope: Option<nanika_host::ExtensionResourceScope>,
    pub(crate) extension_icon_url: String,
    pub(crate) instance_id: u64,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) revision: u64,
    pub(crate) view: std::sync::Arc<nanika_protocol::View>
}

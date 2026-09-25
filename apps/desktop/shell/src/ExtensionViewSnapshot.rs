use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionViewSnapshot {
    pub(crate) route_id: u64,
    pub(crate) extension_id: String,
    pub(crate) instance_id: u64,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) revision: u64,
    pub(crate) view: std::sync::Arc<nanika_protocol::View>,
}

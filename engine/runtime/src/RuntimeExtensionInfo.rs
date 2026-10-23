/// Validated presentation metadata, available without activating an extension.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExtensionInfo {
    pub id: String,
    pub name: nanika_protocol::LocalizedText,
    #[serde(skip)]
    pub name_search_values: Vec<String>,
    #[serde(skip)]
    pub configuration_search_values: std::collections::BTreeMap<String, Vec<String>>,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_scope: Option<crate::ExtensionResourceScope>,
    pub enabled: bool,
    pub pending: bool,
    pub state: crate::RuntimeExtensionState,
    pub instance_id: Option<u64>,
    pub lifecycle_error: Option<crate::LifecycleError>,
    pub configuration_error: Option<crate::OperationError>,
    pub icon: String,
    #[serde(skip)]
    pub icon_hash: Option<String>
}

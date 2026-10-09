/// Validated presentation metadata, available without activating an extension.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExtensionInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub pending: bool,
    pub state: crate::RuntimeExtensionState,
    pub instance_id: Option<u64>,
    pub lifecycle_error: Option<String>,
    pub configuration_error: Option<String>,
    pub icon: String,
    #[serde(skip)]
    pub icon_hash: Option<String>
}

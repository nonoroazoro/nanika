#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfigurationMenuRequest {
    pub session_id: u64,
    pub route_id: u64,
    pub instance_id: u64
}

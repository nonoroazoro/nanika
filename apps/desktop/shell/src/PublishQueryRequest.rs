use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishQueryRequest {
    pub(crate) session_id: u64,
    pub(crate) request_id: u64,
    pub(crate) query: String,
}

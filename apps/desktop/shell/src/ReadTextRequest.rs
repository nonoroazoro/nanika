use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadTextRequest {
    pub(crate) session_id: u64,
    pub(crate) route_id: u64,
    pub(crate) read: nanika_protocol::TextRead
}

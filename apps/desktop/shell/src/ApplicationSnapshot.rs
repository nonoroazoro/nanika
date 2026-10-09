use serde::Serialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationSnapshot {
    pub(crate) session_id: u64,
    pub(crate) version: &'static str,
    pub(crate) locale: String,
    pub(crate) platform: nanika_platform::TargetPlatform,
    pub(crate) max_query_chars: usize,
    pub(crate) resource_origin: String
}

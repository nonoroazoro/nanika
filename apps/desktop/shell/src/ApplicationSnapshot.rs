use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationSnapshot {
    pub(crate) session_id: u64,
    pub(crate) version: &'static str,
    pub(crate) locale: String,
    pub(crate) max_query_chars: usize,
    pub(crate) resource_origin: String,
}

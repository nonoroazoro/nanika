use serde::Serialize;

use crate::{SearchPhase, SearchResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RootSearchSnapshot {
    pub(crate) navigation: crate::NavigationSnapshot,
    pub(crate) session_id: u64,
    pub(crate) request_id: u64,
    pub(crate) revision: u64,
    pub(crate) query: String,
    pub(crate) results: Vec<SearchResult>,
    pub(crate) phase: SearchPhase,
    pub(crate) error: Option<String>,
    pub(crate) warnings: Vec<String>,
}

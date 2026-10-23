use serde::Serialize;

use crate::{SearchPhase, SearchResult};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RootSearchSnapshot {
    pub(crate) text_invalidations: Vec<crate::TextInvalidationNotice>,
    pub(crate) navigation: crate::NavigationSnapshot,
    pub(crate) session_id: u64,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) localization: Option<nanika_localization::LocalizationSnapshot>,
    pub(crate) request_id: u64,
    pub(crate) revision: u64,
    pub(crate) query: String,
    pub(crate) result_revision: u64,
    pub(crate) result_offset: usize,
    pub(crate) total_results: usize,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) results: Option<Vec<SearchResult>>,
    pub(crate) phase: SearchPhase,
    pub(crate) error: Option<nanika_host::OperationError>,
    pub(crate) warnings: Vec<nanika_host::RuntimeWarning>,
    pub(crate) pending_extensions: Vec<String>
}

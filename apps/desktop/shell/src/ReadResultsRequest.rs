use serde::Deserialize;

/// A viewport request bound to an immutable search result revision.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadResultsRequest {
    pub(crate) session_id: u64,
    pub(crate) request_id: u64,
    pub(crate) result_revision: u64,
    pub(crate) range_id: u64,
    pub(crate) offset: usize,
    pub(crate) count: usize
}

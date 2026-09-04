use serde::Serialize;

use crate::RootSearchSnapshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationSnapshot {
    pub(crate) session_id: u64,
    pub(crate) locale: String,
    pub(crate) root_search: RootSearchSnapshot,
}

use serde::Serialize;

use crate::ExtensionViewSnapshot;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationSnapshot {
    pub(crate) revision: u64,
    /// Omitted means unchanged; null explicitly returns to Root Search.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) current: Option<Option<ExtensionViewSnapshot>>,
    pub(crate) busy: bool,
    pub(crate) error: Option<String>,
    pub(crate) dismiss_count: u64
}

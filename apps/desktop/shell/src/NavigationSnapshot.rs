use serde::Serialize;

use crate::ExtensionViewSnapshot;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationSnapshot {
    pub(crate) revision: u64,
    pub(crate) current: Option<ExtensionViewSnapshot>,
    pub(crate) busy: bool,
    pub(crate) error: Option<String>,
    pub(crate) dismiss_count: u64,
}

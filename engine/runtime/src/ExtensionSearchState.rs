use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::{
    ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery, ExtensionSettingsUpdate,
    ExtensionViewRequest,
};

#[derive(Debug, Default)]
pub(crate) struct ExtensionSearchState {
    pub(crate) closed: bool,
    pub(crate) latest_query: Option<ExtensionSearchQuery>,
    pub(crate) query: Option<ExtensionSearchQuery>,
    pub(crate) refresh: Option<ExtensionRefresh>,
    pub(crate) invocations: VecDeque<ExtensionInvocation>,
    pub(crate) view_events: VecDeque<ExtensionViewRequest>,
    pub(crate) active_invocation_id: Option<u64>,
    pub(crate) cancelled_invocations: HashSet<u64>,
    pub(crate) settings: VecDeque<ExtensionSettingsUpdate>,
    pub(crate) shutdown: Arc<AtomicBool>,
}

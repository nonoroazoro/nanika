use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::{
    ExtensionConfigurationUpdate, ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery,
    ExtensionViewRequest,
};

#[derive(Debug, Default)]
pub(crate) struct ExtensionSearchState {
    pub(crate) lifecycle: crate::RuntimeExtensionState,
    pub(crate) lifecycle_error: Option<String>,
    pub(crate) closed: bool,
    pub(crate) finished: bool,
    pub(crate) stop_result: Option<Result<(), String>>,
    pub(crate) latest_query: Option<ExtensionSearchQuery>,
    pub(crate) query: Option<ExtensionSearchQuery>,
    pub(crate) entry_preparation: Option<(u64, Vec<String>)>,
    pub(crate) refreshes: VecDeque<ExtensionRefresh>,
    pub(crate) invocations: VecDeque<ExtensionInvocation>,
    pub(crate) view_events: VecDeque<ExtensionViewRequest>,
    pub(crate) active_invocation_id: Option<u64>,
    pub(crate) cancelled_invocations: HashSet<u64>,
    pub(crate) configurations: VecDeque<ExtensionConfigurationUpdate>,
    pub(crate) configuration_pending: bool,
    pub(crate) shutdown: Arc<AtomicBool>,
}

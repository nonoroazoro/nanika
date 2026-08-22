use std::collections::VecDeque;

use crate::{ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery, ExtensionSettingsUpdate};

#[derive(Debug, Default)]
pub(crate) struct ExtensionSearchState {
    pub(crate) query: Option<ExtensionSearchQuery>,
    pub(crate) refresh: Option<ExtensionRefresh>,
    pub(crate) invocations: VecDeque<ExtensionInvocation>,
    pub(crate) settings: VecDeque<ExtensionSettingsUpdate>,
    pub(crate) shutdown: bool,
}

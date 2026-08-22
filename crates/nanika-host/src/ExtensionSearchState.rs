use std::collections::VecDeque;

use crate::{ExtensionInvocation, ExtensionSearchQuery};

#[derive(Debug, Default)]
pub(crate) struct ExtensionSearchState {
    pub(crate) query: Option<ExtensionSearchQuery>,
    pub(crate) invocations: VecDeque<ExtensionInvocation>,
    pub(crate) shutdown: bool,
}

use nanika_protocol::{NavigationEffect, View};

/// Validated extension view response delivered across the runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeViewCompletion {
    pub revision: u64,
    pub effect: NavigationEffect,
    pub view: Option<View>,
}

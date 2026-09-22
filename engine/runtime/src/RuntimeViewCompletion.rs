use nanika_protocol::{NavigationEffect, View};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeViewCompletion {
    pub revision: u64,
    pub effect: NavigationEffect,
    pub view: Option<View>,
}

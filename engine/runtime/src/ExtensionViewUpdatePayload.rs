use nanika_protocol::{NavigationEffect, View};

/// Validated view state returned by an extension interaction.
#[derive(Debug, Clone)]
pub(crate) struct ExtensionViewUpdatePayload {
    pub(crate) revision: u64,
    pub(crate) effect: NavigationEffect,
    pub(crate) view: Option<View>,
}

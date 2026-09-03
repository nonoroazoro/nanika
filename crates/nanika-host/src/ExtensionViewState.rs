use nanika_protocol::View;

/// Host navigation state for one extension-owned view session.
#[derive(Debug, Clone)]
pub(crate) struct ExtensionViewState {
    pub(crate) extension_id: String,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) revision: u64,
    pub(crate) view: View,
    pub(crate) search_text: Option<String>,
    pub(crate) queued_search_text: Option<String>,
    pub(crate) selected_item_id: Option<String>,
    pub(crate) queued_selection_item_id: Option<String>,
}

/// Route-local interaction state for the clipboard history view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardViewState {
    pub query: String,
    pub selected_item_id: Option<String>,
    pub content_type: String,
    pub visible_limit: usize,
    pub revision: u64,
}

impl ClipboardViewState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected_item_id: None,
            content_type: "all".to_owned(),
            visible_limit: 100,
            revision: 1,
        }
    }
}

impl Default for ClipboardViewState {
    fn default() -> Self {
        Self::new()
    }
}

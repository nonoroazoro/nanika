pub const CLIPBOARD_BATCH_SIZE: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardViewState {
    pub query: String,
    pub selected_item_id: Option<String>,
    pub content_type: String,
    pub offset: usize,
    pub count: usize,
    pub anchor_id: Option<String>,
    pub text_chunk: usize,
    pub revision: u64,
}

impl ClipboardViewState {
    /// Replace the result scope with a bounded window sized for the host viewport.
    pub fn reset_results(&mut self, minimum_items: std::num::NonZeroU32) {
        self.count = minimum_items.get() as usize;
        self.offset = 0;
        self.anchor_id = None;
        self.selected_item_id = None;
        self.text_chunk = 0;
    }

    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected_item_id: None,
            content_type: "all".to_owned(),
            offset: 0,
            count: CLIPBOARD_BATCH_SIZE,
            anchor_id: None,
            text_chunk: 0,
            revision: 1,
        }
    }
}

impl Default for ClipboardViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Only the active viewport's summaries are cached; selection does not reread that window.
pub(crate) struct ClipboardWindow {
    pub collection_id: String,
    pub offset: usize,
    pub count: usize,
    pub items: Vec<crate::ClipboardItem>,
}

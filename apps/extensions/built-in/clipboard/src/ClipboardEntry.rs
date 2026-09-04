use nanika_protocol::ClipboardContent;

/// One persisted clipboard history item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardEntry {
    pub entry_id: String,
    pub content_hash: String,
    pub title: String,
    pub content: ClipboardContent,
    pub byte_size: u64,
    pub captured_at: u64,
    pub pinned: bool,
}

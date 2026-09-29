/// One selected payload and its UTF-8 chunk boundaries. Index once, slice in constant time.
pub(crate) struct ClipboardPreview {
    pub entry: crate::ClipboardEntry,
    pub chunks: Vec<std::ops::Range<usize>>,
}

impl ClipboardPreview {
    pub fn new(entry: crate::ClipboardEntry) -> Self {
        let mut chunks = Vec::new();
        if let nanika_protocol::ClipboardContent::Text { value } = &entry.content {
            let mut start = 0;
            for (index, (offset, _)) in value.char_indices().enumerate() {
                if index > 0 && index % nanika_protocol::DETAIL_TEXT_BATCH_CHARS == 0 {
                    chunks.push(start..offset);
                    start = offset;
                }
            }
            chunks.push(start..value.len());
        }
        Self { entry, chunks }
    }
}

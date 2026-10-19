/// One immutable selected payload. Readers share its index without querying storage.
pub(crate) struct ClipboardPreview {
    pub entry: crate::ClipboardEntry,
    pub document: Option<nanika_protocol::TextDocumentIndex>
}

impl ClipboardPreview {
    pub fn new(entry: crate::ClipboardEntry, revision: String) -> Result<Self, String> {
        let document = if let nanika_protocol::ClipboardContent::Text { value } = &entry.content {
            Some(nanika_protocol::TextDocumentIndex::new(
                entry.entry_id.clone(),
                revision,
                value
            )?)
        } else {
            None
        };
        Ok(Self { entry, document })
    }

    pub fn read(
        &self,
        request: &nanika_protocol::TextRead
    ) -> Result<nanika_protocol::TextChunk, String> {
        let nanika_protocol::ClipboardContent::Text { value } = &self.entry.content else {
            return Err("Selected content is not text.".to_owned());
        };
        self.document
            .as_ref()
            .ok_or("Text document is unavailable.")?
            .read(value, request)
    }
}

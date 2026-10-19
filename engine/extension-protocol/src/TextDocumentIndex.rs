/// Publisher-owned UTF-8 boundaries and UTF-16 metadata, computed in one source pass.
/// The index borrows no source and does not duplicate the document payload.
#[derive(Debug)]
pub struct TextDocumentIndex {
    pub document: crate::TextDocument,
    _ranges: Vec<std::ops::Range<usize>>
}

impl TextDocumentIndex {
    pub fn new(id: String, revision: String, text: &str) -> Result<Self, String> {
        let mut ranges = Vec::new();
        let mut start = 0;
        let mut length = 0;
        let mut line_length: usize = 0;
        let mut maximum = 0;
        let mut lines = 1;
        let mut carriage_return = false;
        for (count, (offset, character)) in text.char_indices().enumerate() {
            if count > 0 && count % crate::DETAIL_TEXT_BATCH_CHARS == 0 {
                if ranges.len() + 1 >= crate::MAX_DETAIL_TEXT_CHUNKS {
                    return Err("text document exceeds the delivery budget".to_owned());
                }
                ranges.push(start..offset);
                start = offset;
            }
            length += character.len_utf16();
            if character == '\n' {
                maximum = maximum.max(line_length.saturating_sub(usize::from(carriage_return)));
                line_length = 0;
                lines += 1;
            } else {
                line_length += character.len_utf16();
            }
            carriage_return = character == '\r';
        }
        ranges.push(start..text.len());
        maximum = maximum.max(line_length);
        let document = crate::TextDocument {
            id,
            revision,
            utf16_length: length,
            line_count: lines,
            max_line_length: maximum,
            total_chunks: ranges.len()
        };
        document.validate()?;
        Ok(Self {
            document,
            _ranges: ranges
        })
    }

    /// The caller owns the immutable source/revision pair and its lifecycle.
    pub fn read(
        &self,
        source: &str,
        request: &crate::TextRead
    ) -> Result<crate::TextChunk, String> {
        if !request.matches(&self.document) {
            return Err("text document changed or read is outside its bounds".to_owned());
        }
        let range = self
            ._ranges
            .get(request.index)
            .ok_or("text chunk is unavailable")?;
        let value = source
            .get(range.clone())
            .ok_or("text source does not match its index")?;
        Ok(crate::TextChunk {
            document_id: self.document.id.clone(),
            revision: self.document.revision.clone(),
            index: request.index,
            value: value.to_owned()
        })
    }
}

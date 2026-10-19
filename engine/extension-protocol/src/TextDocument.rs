use serde::{Deserialize, Serialize};

/// Immutable source metadata. The publisher must change revision whenever bytes change.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextDocument {
    pub id: String,
    pub revision: String,
    pub utf16_length: usize,
    pub line_count: usize,
    pub max_line_length: usize,
    pub total_chunks: usize
}

impl TextDocument {
    pub fn validate(&self) -> Result<(), String> {
        crate::view::validate_id("text document identity", &self.id)?;
        crate::view::validate_id("text document revision", &self.revision)?;
        let maximum = crate::DETAIL_TEXT_BATCH_CHARS * crate::MAX_DETAIL_TEXT_CHUNKS * 2;
        if self.total_chunks == 0
            || self.total_chunks > crate::MAX_DETAIL_TEXT_CHUNKS
            || self.utf16_length > maximum
            || self.line_count == 0
            || self.line_count > self.utf16_length + 1
            || self.max_line_length > self.utf16_length
            || (self.total_chunks > 1
                && self.utf16_length <= (self.total_chunks - 1) * crate::DETAIL_TEXT_BATCH_CHARS)
            || self.utf16_length > self.total_chunks * crate::DETAIL_TEXT_BATCH_CHARS * 2
        {
            return Err("text document metadata is invalid".to_owned());
        }
        Ok(())
    }
}

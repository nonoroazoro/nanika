use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextChunk {
    pub document_id: String,
    pub revision: String,
    pub index: usize,
    pub value: String
}

impl TextChunk {
    pub fn validate_for(&self, read: &crate::TextRead) -> Result<(), String> {
        read.validate()?;
        if self.document_id != read.document_id
            || self.revision != read.revision
            || self.index != read.index
            || self.value.chars().count() > crate::DETAIL_TEXT_BATCH_CHARS
        {
            return Err("text response does not match its bounded read".to_owned());
        }
        Ok(())
    }
}

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextRead {
    pub document_id: String,
    pub revision: String,
    pub index: usize
}

impl TextRead {
    pub fn validate(&self) -> Result<(), String> {
        crate::view::validate_id("text document identity", &self.document_id)?;
        crate::view::validate_id("text document revision", &self.revision)?;
        if self.index >= crate::MAX_DETAIL_TEXT_CHUNKS {
            return Err("text read exceeds the document limit".to_owned());
        }
        Ok(())
    }

    pub fn matches(&self, document: &crate::TextDocument) -> bool {
        self.document_id == document.id
            && self.revision == document.revision
            && self.index < document.total_chunks
    }
}

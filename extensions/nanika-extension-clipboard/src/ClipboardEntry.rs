use nanika_protocol::{Candidate, ClipboardContent};

use crate::RESTORE_ACTION_ID;

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

impl ClipboardEntry {
    pub fn candidate(&self) -> Candidate {
        let aliases = match &self.content {
            ClipboardContent::Text { value } => {
                vec![value.chars().take(512).collect()]
            }
            ClipboardContent::Files { paths } => paths
                .iter()
                .take(16)
                .map(|path| path.chars().take(512).collect())
                .collect(),
            ClipboardContent::PngFile { .. } => Vec::new(),
        };
        Candidate {
            entry_id: self.entry_id.clone(),
            title: self.title.clone(),
            action_id: RESTORE_ACTION_ID.to_owned(),
            aliases,
        }
    }
}

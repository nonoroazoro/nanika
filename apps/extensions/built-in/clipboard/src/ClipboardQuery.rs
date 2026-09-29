use crate::{ClipboardEntry, ClipboardQueryEntry};
use nanika_protocol::ClipboardContent;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Only the active query's ordering metadata and immutable reviewed IDs are retained.
pub(crate) struct ClipboardQuery {
    pub text: String,
    pub content_type: String,
    pub entries: Vec<ClipboardQueryEntry>,
    pub entry_ids: Arc<Vec<String>>,
    pub positions: HashMap<String, usize>,
}

impl ClipboardQuery {
    pub fn apply(&mut self, upsert: Option<&ClipboardEntry>, removed: &HashSet<String>) -> bool {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| {
            !removed.contains(&entry.entry_id)
                && upsert.is_none_or(|updated| updated.entry_id != entry.entry_id)
        });
        let mut changed = self.entries.len() != previous_len;
        if let Some(entry) =
            upsert.filter(|entry| !removed.contains(&entry.entry_id) && self._matches(entry))
        {
            // Keep the same timestamp saturation and tie-break order as SQLite.
            let captured_at = entry.captured_at.min(i64::MAX as u64) as i64;
            let index = self.entries.partition_point(|current| {
                current.captured_at > captured_at
                    || (current.captured_at == captured_at && current.entry_id < entry.entry_id)
            });
            self.entries.insert(
                index,
                ClipboardQueryEntry {
                    entry_id: entry.entry_id.clone(),
                    captured_at,
                },
            );
            changed = true;
        }
        if changed {
            // Previously published IDs are also reviewed clear scopes and must never mutate.
            self.positions = self
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (entry.entry_id.clone(), index))
                .collect();
            self.entry_ids = Arc::new(
                self.entries
                    .iter()
                    .map(|entry| entry.entry_id.clone())
                    .collect(),
            );
        }
        changed
    }

    fn _matches(&self, entry: &ClipboardEntry) -> bool {
        let content_type = match entry.content {
            ClipboardContent::Text { .. } => "text",
            ClipboardContent::Files { .. } => "files",
            ClipboardContent::PngFile { .. } => "images",
        };
        (self.content_type == "all" || self.content_type == content_type)
            && crate::query::matches(&self.text, &entry.title, &entry.content)
    }
}

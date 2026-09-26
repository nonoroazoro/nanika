use crate::ClipboardEntry;
use std::collections::HashSet;
use std::path::PathBuf;

/// A committed mutation; filesystem cleanup cannot undo its visible database state.
#[derive(Debug)]
pub struct ClipboardChange {
    pub removed: HashSet<String>,
    pub retained_images: HashSet<PathBuf>,
}

impl ClipboardChange {
    /// Preserve all untouched payload allocations and the database ordering contract.
    pub fn apply(&self, entries: &mut Vec<ClipboardEntry>, updated: Option<ClipboardEntry>) {
        entries.retain(|entry| {
            !self.removed.contains(&entry.entry_id)
                && updated
                    .as_ref()
                    .is_none_or(|next| next.entry_id != entry.entry_id)
        });
        if let Some(updated) = updated.filter(|entry| !self.removed.contains(&entry.entry_id)) {
            let position = entries.partition_point(|entry| {
                entry.captured_at > updated.captured_at
                    || (entry.captured_at == updated.captured_at
                        && entry.entry_id < updated.entry_id)
            });
            entries.insert(position, updated);
        }
    }
}

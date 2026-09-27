/// Ordering metadata for a matching entry; payloads remain in SQLite.
#[derive(Clone)]
pub(crate) struct ClipboardQueryEntry {
    pub entry_id: String,
    pub captured_at: i64,
}

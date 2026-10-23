/// A list row does not retain a text or image payload.
#[derive(Debug, Clone)]
pub(crate) struct ClipboardItem {
    pub entry_id: String,
    pub summary: crate::ClipboardSummary
}

impl ClipboardItem {
    pub fn from_entry(entry: &crate::ClipboardEntry) -> Result<Self, String> {
        Ok(Self {
            entry_id: entry.entry_id.clone(),
            summary: entry.summary()?
        })
    }
}

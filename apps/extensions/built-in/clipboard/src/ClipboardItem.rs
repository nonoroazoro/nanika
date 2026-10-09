/// A list row does not retain a text or image payload.
#[derive(Debug, Clone)]
pub(crate) struct ClipboardItem {
    pub entry_id: String,
    pub title: String,
    pub kind: String,
    pub first_path: Option<String>
}

impl ClipboardItem {
    pub fn from_entry(entry: &crate::ClipboardEntry) -> Self {
        let (kind, first_path) = match &entry.content {
            nanika_protocol::ClipboardContent::Text { .. } => ("text", None),
            nanika_protocol::ClipboardContent::PngFile { .. } => ("image", None),
            nanika_protocol::ClipboardContent::Files { paths } => ("files", paths.first().cloned())
        };
        Self {
            entry_id: entry.entry_id.clone(),
            title: entry.title.clone(),
            kind: kind.into(),
            first_path
        }
    }
}

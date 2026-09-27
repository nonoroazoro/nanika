/// A list row does not retain a text or image payload.
#[derive(Debug, Clone)]
pub(crate) struct ClipboardItem {
    pub entry_id: String,
    pub title: String,
    pub kind: String,
    pub first_path: Option<String>,
}

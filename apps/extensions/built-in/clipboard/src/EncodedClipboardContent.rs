/// SQLite columns derived from one typed clipboard payload.
pub(crate) struct EncodedClipboardContent {
    pub(crate) kind: &'static str,
    pub(crate) text: Option<String>,
    pub(crate) files: Option<String>,
    pub(crate) image: Option<String>,
}

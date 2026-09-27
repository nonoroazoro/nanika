/// Stable host-owned input; no producer path is dereferenced after admission.
pub(crate) enum PreparedClipboardContent {
    Text(String),
    Files(Vec<String>),
    Png(Vec<u8>),
}

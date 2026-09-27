use crate::{ClipboardWritePermit, PreparedClipboardContent};

/// Validated resource ownership and its bounded memory reservation.
pub struct PreparedClipboardWrite {
    pub(crate) content: PreparedClipboardContent,
    pub(crate) permit: ClipboardWritePermit,
}

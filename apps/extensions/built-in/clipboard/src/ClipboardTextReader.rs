use std::sync::{Arc, Mutex};

/// Bounded source reads share an immutable preview without entering the database
/// command queue. The owner revokes this handle before publishing invalidation.
#[derive(Clone, Default)]
pub struct ClipboardTextReader {
    _preview: Arc<Mutex<Option<Arc<crate::ClipboardPreview>>>>
}

impl ClipboardTextReader {
    pub fn read(
        &self,
        request: &nanika_protocol::TextRead
    ) -> Result<nanika_protocol::TextChunk, String> {
        request.validate()?;
        let preview = self
            ._preview
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .ok_or("Text source is no longer available.")?;
        preview.read(request)
    }

    pub(crate) fn publish(&self, preview: Option<Arc<crate::ClipboardPreview>>) {
        *self
            ._preview
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = preview;
    }
}

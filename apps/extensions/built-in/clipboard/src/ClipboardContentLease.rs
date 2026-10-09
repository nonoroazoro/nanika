use std::sync::mpsc::SyncSender;

use nanika_protocol::ClipboardContent;

use crate::ClipboardCommand;

/// Keeps a stored image alive until the host has finished consuming the copy request.
/// Borrowing the owner prevents shutdown while a copy still holds its payload.
pub struct ClipboardContentLease<'a> {
    content: ClipboardContent,
    commands: &'a SyncSender<ClipboardCommand>
}

impl<'a> ClipboardContentLease<'a> {
    pub(crate) fn new(
        content: ClipboardContent,
        commands: &'a SyncSender<ClipboardCommand>
    ) -> Self {
        Self { content, commands }
    }

    pub fn content(&self) -> &ClipboardContent {
        &self.content
    }
}

impl Drop for ClipboardContentLease<'_> {
    fn drop(&mut self) {
        if let ClipboardContent::PngFile { path } = &self.content
            && self
                .commands
                .send(ClipboardCommand::ReleaseContent { path: path.into() })
                .is_err()
        {
            eprintln!("clipboard owner closed before releasing the copy payload");
        }
    }
}

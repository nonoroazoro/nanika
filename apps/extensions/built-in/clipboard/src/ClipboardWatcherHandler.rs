use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;

use clipboard_rs::ClipboardHandler;

use crate::ClipboardCommand;

/// Minimal native watcher callback that never performs clipboard I/O.
pub(crate) struct ClipboardWatcherHandler {
    pub(crate) commands: SyncSender<ClipboardCommand>,
    pub(crate) suppress_next_change: Arc<AtomicBool>,
}

impl ClipboardHandler for ClipboardWatcherHandler {
    fn on_clipboard_change(&mut self) {
        if self.suppress_next_change.swap(false, Ordering::AcqRel) {
            return;
        }
        if self
            .commands
            .send(ClipboardCommand::Capture { response: None })
            .is_err()
        {
            eprintln!("clipboard capture owner closed while delivering a change event");
        }
    }
}

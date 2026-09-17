use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use clipboard_rs::ClipboardHandler;

use crate::{ClipboardCaptureGate, ClipboardCommand};

/// Minimal native watcher callback that never performs clipboard I/O.
pub(crate) struct ClipboardWatcherHandler {
    pub(crate) commands: SyncSender<ClipboardCommand>,
    pub(crate) capture_gate: Arc<Mutex<ClipboardCaptureGate>>,
}

impl ClipboardHandler for ClipboardWatcherHandler {
    fn on_clipboard_change(&mut self) {
        let revision = nanika_platform::clipboard_revision();
        let should_capture = self
            .capture_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .observe(revision);
        if !should_capture {
            return;
        }
        if self.commands.send(ClipboardCommand::Capture).is_err() {
            eprintln!("clipboard capture owner closed while delivering a change event");
        }
    }
}

use std::sync::mpsc::SyncSender;

use clipboard_rs::ClipboardHandler;

use crate::ClipboardCommand;

/// Minimal native watcher callback that never performs clipboard I/O.
pub(crate) struct ClipboardWatcherHandler {
    pub(crate) commands: SyncSender<ClipboardCommand>,
}

impl ClipboardHandler for ClipboardWatcherHandler {
    fn on_clipboard_change(&mut self) {
        if self
            .commands
            .send(ClipboardCommand::Capture { response: None })
            .is_err()
        {
            eprintln!("clipboard capture owner closed while delivering a change event");
        }
    }
}

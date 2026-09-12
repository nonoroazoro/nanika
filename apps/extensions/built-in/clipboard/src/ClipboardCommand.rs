use std::sync::mpsc::SyncSender;

pub(crate) enum ClipboardCommand {
    Capture {
        response: Option<SyncSender<Result<(), String>>>,
    },
    Clear {
        response: SyncSender<Result<(), String>>,
    },
    Shutdown,
}

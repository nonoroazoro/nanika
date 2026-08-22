use std::sync::mpsc::SyncSender;

pub(crate) enum ClipboardCommand {
    Capture {
        response: Option<SyncSender<Result<(), String>>>,
    },
    MarkUsed {
        entry_id: String,
        used_at: u64,
    },
    Shutdown,
}

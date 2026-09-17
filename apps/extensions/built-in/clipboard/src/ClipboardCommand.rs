use std::sync::mpsc::SyncSender;

use crate::ClipboardConfig;

pub(crate) enum ClipboardCommand {
    Capture,
    Clear {
        entry_ids: Vec<String>,
        response: SyncSender<Result<(), String>>,
    },
    ApplyRetention {
        config: ClipboardConfig,
        response: SyncSender<Result<(), String>>,
    },
    Shutdown,
}

use std::sync::mpsc::SyncSender;

use crate::ClipboardConfig;

pub(crate) enum ClipboardCommand {
    Capture {
        response: Option<SyncSender<Result<(), String>>>,
    },
    Clear {
        response: SyncSender<Result<(), String>>,
    },
    ApplyRetention {
        config: ClipboardConfig,
        response: SyncSender<Result<(), String>>,
    },
    Shutdown,
}

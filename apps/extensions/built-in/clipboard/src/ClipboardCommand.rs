use std::sync::mpsc::SyncSender;

use crate::ClipboardConfig;

pub(crate) enum ClipboardCommand {
    Capture,
    Clear {
        entry_ids: std::sync::Arc<Vec<String>>,
        response: SyncSender<Result<(), String>>,
    },
    ApplyRetention {
        config: ClipboardConfig,
        response: SyncSender<Result<(), String>>,
    },
    Present {
        state: crate::ClipboardViewState,
        expected_revision: Option<u64>,
        response: SyncSender<Result<crate::ClipboardPresentation, String>>,
    },
    Content {
        entry_id: String,
        response: SyncSender<Result<nanika_protocol::ClipboardContent, String>>,
    },
    ReleaseContent {
        path: std::path::PathBuf,
    },
    Shutdown,
}

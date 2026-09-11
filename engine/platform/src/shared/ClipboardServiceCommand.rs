use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

use nanika_protocol::{ClipboardContent, HostServiceResponse};

pub(crate) enum ClipboardServiceCommand {
    Write {
        content: ClipboardContent,
        payload_root: Option<PathBuf>,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    Shutdown,
}

use std::path::PathBuf;
use std::sync::mpsc::SyncSender;
use std::time::Instant;

use nanika_protocol::{ClipboardContent, HostServiceResponse};

pub(crate) enum ClipboardServiceCommand {
    Write {
        content: ClipboardContent,
        payload_root: Option<PathBuf>,
        deadline: Instant,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    Shutdown,
}

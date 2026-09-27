use std::sync::mpsc::SyncSender;

use nanika_protocol::HostServiceResponse;

pub(crate) enum ClipboardServiceCommand {
    Write {
        prepared: crate::PreparedClipboardWrite,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    Shutdown,
}

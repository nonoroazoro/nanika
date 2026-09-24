use std::sync::mpsc::SyncSender;

use nanika_protocol::{HostServiceResponse, LaunchDescriptor};

pub(crate) enum LauncherCommand {
    Reveal {
        path: String,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    Launch {
        descriptor: LaunchDescriptor,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    #[cfg(windows)]
    Shutdown,
}

use std::sync::mpsc::SyncSender;

use nanika_protocol::{HostServiceResponse, LaunchDescriptor};

pub(crate) enum LauncherCommand {
    Launch {
        descriptor: LaunchDescriptor,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    #[cfg(not(target_os = "macos"))]
    Shutdown,
}

use std::sync::mpsc::SyncSender;
use std::time::Instant;

use nanika_protocol::{HostServiceResponse, LaunchDescriptor};

pub(crate) enum LauncherCommand {
    Launch {
        descriptor: LaunchDescriptor,
        deadline: Instant,
        response: SyncSender<Result<HostServiceResponse, String>>,
    },
    Shutdown,
}

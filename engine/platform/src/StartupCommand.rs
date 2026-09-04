use std::sync::mpsc::SyncSender;

use crate::{PlatformError, StartupStatus};

pub(crate) enum StartupCommand {
    Query {
        response: SyncSender<Result<StartupStatus, PlatformError>>,
    },
    SetEnabled {
        enabled: bool,
        response: SyncSender<Result<StartupStatus, PlatformError>>,
    },
    Shutdown,
}

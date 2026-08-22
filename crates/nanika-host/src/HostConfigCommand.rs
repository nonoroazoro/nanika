use std::sync::mpsc::SyncSender;

use crate::HostConfig;

pub(crate) enum HostConfigCommand {
    Update {
        hotkey: String,
        reduced_motion: bool,
        response: SyncSender<Result<HostConfig, String>>,
    },
    Shutdown,
}

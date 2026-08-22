use std::sync::mpsc::Receiver;

use global_hotkey::hotkey::HotKey;

use crate::HostConfig;

pub(crate) struct PendingHostSettings {
    pub(crate) response: Receiver<Result<HostConfig, String>>,
    pub(crate) previous_hotkey: Option<HotKey>,
    pub(crate) new_registration: bool,
}

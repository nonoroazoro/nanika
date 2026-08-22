use std::sync::mpsc::SyncSender;

use crate::PlatformEvent;

pub(crate) struct MacMenuTargetIvars {
    pub(crate) events: SyncSender<PlatformEvent>,
}

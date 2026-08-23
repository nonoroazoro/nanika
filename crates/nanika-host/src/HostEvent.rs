use std::time::Instant;

use global_hotkey::GlobalHotKeyEvent;
use nanika_platform::PlatformEvent;

pub(crate) enum HostEvent {
    Hotkey {
        event: GlobalHotKeyEvent,
        received_at: Instant,
    },
    Platform {
        event: PlatformEvent,
        received_at: Instant,
    },
}

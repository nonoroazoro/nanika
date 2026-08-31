use std::time::{Duration, Instant};

use global_hotkey::GlobalHotKeyEvent;
use nanika_platform::PlatformEvent;

pub(crate) enum HostEvent {
    Hotkey {
        event: GlobalHotKeyEvent,
        received_at: Instant,
        delivery_delay: Option<Duration>,
    },
    Platform {
        event: PlatformEvent,
        received_at: Instant,
    },
}

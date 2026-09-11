mod fatal_error;
pub(crate) mod hotkey_timing;
#[path = "HotkeyTimingObserver.rs"]
mod hotkey_timing_observer;
mod instance;
mod overlay_position;
pub(crate) mod process_launch;
pub(crate) mod process_launcher;
#[path = "SingleInstance.rs"]
mod single_instance;
pub(crate) mod startup;

pub use fatal_error::report as report_fatal_error;
pub use hotkey_timing_observer::HotkeyTimingObserver;
pub use instance::{acquire as acquire_instance, signal_activate};
pub use overlay_position::active_overlay_position;
pub use single_instance::SingleInstance;
pub(crate) use startup::{set_enabled as set_startup_enabled, status as startup_status};

/// Platform selected by this artifact's compilation target.
pub const fn target_platform() -> &'static str {
    "windows"
}

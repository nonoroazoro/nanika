#[path = "ExtensionProcessTree.rs"]
mod extension_process_tree;
mod fatal_error;
mod filesystem;
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

pub use extension_process_tree::{ExtensionProcessTree, configure_extension_command};
pub use fatal_error::report as report_fatal_error;
pub use filesystem::{atomic_replace, companion_executable, make_executable, open_regular_file};
pub use hotkey_timing_observer::HotkeyTimingObserver;
pub use instance::{acquire as acquire_instance, signal_activate};
pub use overlay_position::active_overlay_position;
pub use single_instance::SingleInstance;
pub(crate) use startup::{set_enabled as set_startup_enabled, status as startup_status};

/// Platform selected by this artifact's compilation target.
pub const fn target_platform() -> &'static str {
    "windows"
}

/// Package target for this artifact, or an explicit unsupported architecture.
pub fn target_triple() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64-pc-windows-msvc",
        _ => "unsupported",
    }
}

pub(crate) mod file_icon;

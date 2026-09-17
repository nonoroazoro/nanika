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

/// Current native pasteboard revision used to distinguish host writes from external copies.
pub fn clipboard_revision() -> u64 {
    use objc2_app_kit::NSPasteboard;

    NSPasteboard::generalPasteboard().changeCount() as u64
}

/// Compare opaque pasteboard revisions while preserving wraparound behavior.
pub fn clipboard_revision_is_after(candidate: u64, baseline: u64) -> bool {
    let distance = candidate.wrapping_sub(baseline);
    distance != 0 && distance < (1_u64 << 63)
}

/// Platform selected by this artifact's compilation target.
pub const fn target_platform() -> &'static str {
    "macos"
}

/// Package target for this artifact, or an explicit unsupported architecture.
pub fn target_triple() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64-apple-darwin",
        "x86_64" => "x86_64-apple-darwin",
        _ => "unsupported",
    }
}

pub(crate) mod file_icon;

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
mod reveal;
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

/// Resolve Windows product roots without exposing the bundle identifier in user paths.
pub fn product_paths(product_name: &str) -> Option<crate::ProductPaths> {
    let base = directories::BaseDirs::new()?;
    let app_data_root = base.data_local_dir().join(product_name);
    let cache_root = app_data_root.join("cache");
    Some(crate::ProductPaths::new(app_data_root, cache_root))
}

/// Current native clipboard sequence used to distinguish host writes from external copies.
pub fn clipboard_revision() -> u64 {
    u64::from(unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() })
}

/// Compare the native 32-bit clipboard sequence while preserving wraparound behavior.
pub fn clipboard_revision_is_after(candidate: u64, baseline: u64) -> bool {
    let distance = (candidate as u32).wrapping_sub(baseline as u32);
    distance != 0 && distance < (1_u32 << 31)
}

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

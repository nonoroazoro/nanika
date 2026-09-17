//! Shared platform services backed by the native adapter selected for this artifact.

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
compile_error!("Nanika release targets are macOS 13+ and Windows 10+ only");

#[cfg(target_os = "macos")]
#[path = "adapters/macos/mod.rs"]
#[allow(unsafe_code)]
mod adapter;
#[cfg(target_os = "windows")]
#[path = "adapters/windows/mod.rs"]
#[allow(unsafe_code)]
mod adapter;

pub use adapter::{
    ExtensionProcessTree, HotkeyTimingObserver, SingleInstance, acquire_instance,
    active_overlay_position, atomic_replace, clipboard_revision, clipboard_revision_is_after,
    companion_executable, configure_extension_command, make_executable, open_regular_file,
    product_paths, report_fatal_error, signal_activate, target_platform, target_triple,
};
pub(crate) use adapter::{set_startup_enabled, startup_status};

#[path = "contracts/InstanceRole.rs"]
mod instance_role;
pub use instance_role::*;
#[path = "contracts/OverlayPosition.rs"]
mod overlay_position;
pub use overlay_position::*;
#[path = "contracts/PlatformError.rs"]
mod platform_error;
pub use platform_error::*;
#[path = "contracts/PlatformEvent.rs"]
mod platform_event;
pub use platform_event::*;
#[path = "contracts/PngResourceError.rs"]
mod png_resource_error;
pub use png_resource_error::*;
#[path = "contracts/StartupStatus.rs"]
mod startup_status;
pub use startup_status::*;
#[path = "contracts/ProductPaths.rs"]
mod product_paths;
pub use product_paths::*;
#[path = "shared/ClipboardService.rs"]
mod clipboard_service;
pub use clipboard_service::*;
#[path = "shared/ClipboardServiceCommand.rs"]
mod clipboard_service_command;
pub(crate) use clipboard_service_command::*;
#[path = "shared/StartupCommand.rs"]
mod startup_command;
pub(crate) use startup_command::*;
#[path = "shared/StartupService.rs"]
mod startup_service;
pub use startup_service::*;
#[path = "shared/LauncherCommand.rs"]
mod launcher_command;
pub(crate) use launcher_command::*;
#[path = "shared/ProcessLauncher.rs"]
mod process_launcher;
pub use process_launcher::*;
#[path = "shared/PngResource.rs"]
mod png_resource;
pub use png_resource::*;
#[path = "shared/hotkey_timing.rs"]
mod hotkey_timing;
pub use hotkey_timing::*;
#[path = "shared/system_locale.rs"]
mod system_locale;
pub use system_locale::*;
#[path = "shared/process_launch.rs"]
mod process_launch;

#[cfg(test)]
#[path = "../tests/HotkeyTiming.rs"]
mod hotkey_timing_tests;
#[cfg(test)]
#[path = "../tests/PngResource.rs"]
mod png_resource_tests;

#[path = "shared/IconNormalizer.rs"]
mod icon_normalizer;
pub use icon_normalizer::normalize_icon_rgba;
#[path = "shared/FileIconCache.rs"]
mod file_icon_cache;
#[path = "shared/image_resize.rs"]
mod image_resize;
#[cfg(any(target_os = "windows", test))]
#[path = "adapters/windows/alpha_recovery.rs"]
mod windows_alpha_recovery;
pub use file_icon_cache::FileIconCache;

#[path = "shared/file_icon.rs"]
mod file_icon;
pub use file_icon::{file_icon_pixels, shell_file_icon_pixels};

#[cfg(test)]
#[path = "../tests/IconNormalizer.rs"]
mod icon_normalizer_tests;

#[cfg(test)]
#[path = "../tests/image_resize.rs"]
mod image_resize_tests;

#[cfg(test)]
#[path = "../tests/windows_alpha_recovery.rs"]
mod windows_alpha_recovery_tests;

#[cfg(test)]
#[path = "../tests/FileIconCache.rs"]
mod file_icon_cache_tests;

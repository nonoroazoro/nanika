//! Platform adapter boundary.

use std::path::Path;

#[path = "ClipboardService.rs"]
mod clipboard_service;
#[path = "ClipboardServiceCommand.rs"]
mod clipboard_service_command;
#[path = "HotkeyRegistration.rs"]
mod hotkey_registration;
#[path = "InstanceRole.rs"]
mod instance_role;
#[path = "LauncherCommand.rs"]
mod launcher_command;
#[path = "NativeMenu.rs"]
mod native_menu;
#[path = "PlatformError.rs"]
mod platform_error;
#[path = "PlatformEvent.rs"]
mod platform_event;
mod process_launch;
#[path = "ProcessLauncher.rs"]
mod process_launcher;
#[path = "SingleInstance.rs"]
#[allow(unsafe_code)]
mod single_instance;
#[path = "StartupCommand.rs"]
mod startup_command;
#[path = "StartupService.rs"]
mod startup_service;
#[path = "StartupStatus.rs"]
mod startup_status;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
#[path = "MacMenuTarget.rs"]
mod mac_menu_target;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
#[path = "MacMenuTargetIvars.rs"]
mod mac_menu_target_ivars;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
#[path = "MacNativeMenu.rs"]
mod mac_native_menu;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos_instance;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod process_launcher_macos;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod startup_macos;
#[cfg(windows)]
mod startup_windows;
#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_instance;

pub use clipboard_service::*;
pub(crate) use clipboard_service_command::*;
pub use hotkey_registration::*;
pub use instance_role::*;
pub(crate) use launcher_command::*;
#[cfg(target_os = "macos")]
pub(crate) use mac_menu_target::*;
#[cfg(target_os = "macos")]
pub(crate) use mac_menu_target_ivars::*;
#[cfg(target_os = "macos")]
pub(crate) use mac_native_menu::*;
pub use native_menu::*;
pub use platform_error::*;
pub use platform_event::*;
pub use process_launcher::*;
pub use single_instance::*;
pub(crate) use startup_command::*;
pub use startup_service::*;
pub use startup_status::*;

pub(crate) fn startup_status(executable: &Path) -> Result<StartupStatus, PlatformError> {
    #[cfg(windows)]
    return startup_windows::status(executable);
    #[cfg(target_os = "macos")]
    return startup_macos::status(executable);
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = executable;
        Err(PlatformError::Unsupported("login startup"))
    }
}

pub(crate) fn set_startup_enabled(
    executable: &Path,
    enabled: bool,
) -> Result<StartupStatus, PlatformError> {
    #[cfg(windows)]
    return startup_windows::set_enabled(executable, enabled);
    #[cfg(target_os = "macos")]
    return startup_macos::set_enabled(executable, enabled);
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (executable, enabled);
        Err(PlatformError::Unsupported("login startup"))
    }
}

/// The target platform selected by the current build.
pub const fn target_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unsupported"
    }
}

/// Acquire the current user's Nanika host instance.
pub fn acquire_instance(
    identity: &str,
    app_data_root: &Path,
) -> Result<InstanceRole, PlatformError> {
    #[cfg(windows)]
    {
        let _ = app_data_root;
        windows_instance::acquire(identity)
    }
    #[cfg(target_os = "macos")]
    {
        let _ = identity;
        macos_instance::acquire(app_data_root)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (identity, app_data_root);
        Err(PlatformError::Unsupported("single instance"))
    }
}

/// Signal an already-running host to activate its overlay.
pub fn signal_activate(identity: &str, app_data_root: &Path) -> Result<(), PlatformError> {
    #[cfg(windows)]
    {
        let _ = app_data_root;
        windows_instance::signal_activate(identity)
    }
    #[cfg(target_os = "macos")]
    {
        let _ = identity;
        macos_instance::signal_activate(app_data_root)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (identity, app_data_root);
        Err(PlatformError::Unsupported("instance activation"))
    }
}

#[cfg(test)]
#[path = "../tests/ClipboardService.rs"]
mod clipboard_service_tests;

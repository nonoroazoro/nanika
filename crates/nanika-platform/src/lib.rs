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
#[path = "PlatformError.rs"]
mod platform_error;
mod process_launch;
#[path = "ProcessLauncher.rs"]
mod process_launcher;
#[path = "SingleInstance.rs"]
#[allow(unsafe_code)]
mod single_instance;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos_instance;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod process_launcher_macos;
#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_instance;

pub use clipboard_service::*;
pub(crate) use clipboard_service_command::*;
pub use hotkey_registration::*;
pub use instance_role::*;
pub(crate) use launcher_command::*;
pub use platform_error::*;
pub use process_launcher::*;
pub use single_instance::*;

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

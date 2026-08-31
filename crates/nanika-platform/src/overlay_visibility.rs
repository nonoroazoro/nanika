use raw_window_handle::RawWindowHandle;

use crate::PlatformError;

/// Apply platform-native overlay visibility behavior.
///
/// Returns whether the native adapter handled the visibility command.
pub fn apply_overlay_visibility(
    window: RawWindowHandle,
    visible: bool,
) -> Result<bool, PlatformError> {
    #[cfg(target_os = "macos")]
    return crate::overlay_visibility_macos::apply(window, visible).map(|()| true);
    #[cfg(windows)]
    {
        let _ = (window, visible);
        Ok(false)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (window, visible);
        Err(PlatformError::Unsupported("overlay visibility"))
    }
}

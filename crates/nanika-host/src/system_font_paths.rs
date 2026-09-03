use std::path::PathBuf;

#[cfg(target_os = "macos")]
#[path = "system_font_paths_macos.rs"]
mod implementation;
#[cfg(windows)]
#[path = "system_font_paths_windows.rs"]
mod implementation;

#[cfg(any(windows, target_os = "macos"))]
pub(crate) fn primary_candidates() -> Vec<PathBuf> {
    implementation::primary_candidates()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) fn primary_candidates() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(any(windows, target_os = "macos"))]
pub(crate) fn fallback_candidates() -> Vec<PathBuf> {
    implementation::fallback_candidates()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) fn fallback_candidates() -> Vec<PathBuf> {
    Vec::new()
}

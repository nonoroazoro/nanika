use std::path::PathBuf;

#[cfg(target_os = "macos")]
#[path = "system_font_paths_macos.rs"]
mod implementation;
#[cfg(windows)]
#[path = "system_font_paths_windows.rs"]
mod implementation;

#[cfg(any(windows, target_os = "macos"))]
pub(crate) fn candidates() -> Vec<(PathBuf, &'static str)> {
    implementation::candidates()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) fn candidates() -> Vec<(PathBuf, &'static str)> {
    Vec::new()
}

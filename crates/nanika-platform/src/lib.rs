//! Platform adapter boundary.

/// Platform-independent adapter error used until native operations are implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformError {
    Unsupported(&'static str),
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

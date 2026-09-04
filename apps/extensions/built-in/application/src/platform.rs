use std::path::{Path, PathBuf};

use crate::{ApplicationEntry, ApplicationError, DiscoveryState};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
#[path = "platform/ShellLinkMetadata.rs"]
mod shell_link_metadata;
#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
use macos as implementation;
#[cfg(not(any(windows, target_os = "macos")))]
use unsupported as implementation;
#[cfg(windows)]
use windows as implementation;

pub(crate) fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
    implementation::standard_roots()
}

pub(crate) fn is_application_path(path: &Path) -> bool {
    implementation::is_application_path(path)
}

pub(crate) fn is_application_bundle(path: &Path) -> bool {
    implementation::is_application_bundle(path)
}

pub(crate) fn read_entry(
    state: &mut DiscoveryState,
    path: &Path,
    seen_at: u64,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    implementation::read_entry(state, path, seen_at, priority)
}

pub(crate) fn extract_icon(
    source: &Path,
    icon_index: i32,
    size: u32,
    target: &Path,
) -> Result<(), ApplicationError> {
    implementation::extract_icon(source, icon_index, size, target)
}

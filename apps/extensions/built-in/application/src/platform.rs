use std::path::{Path, PathBuf};

use crate::{ApplicationEntry, ApplicationError, DiscoveryState};

#[path = "platform/DiscoveryRoots.rs"]
mod discovery_roots;
use discovery_roots::DiscoveryRoots;
#[path = "platform/DiscoveryFailure.rs"]
mod discovery_failure;
use discovery_failure::DiscoveryFailure;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod scoop_shim;
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
    implementation::standard_roots(|_| true).map(|roots| roots.paths)
}

pub(crate) fn shortcut_target(path: &Path) -> Result<String, ApplicationError> {
    implementation::shortcut_target(path)
}

pub(crate) fn configured_roots(
    enabled: &std::collections::BTreeSet<String>,
) -> Result<DiscoveryRoots, ApplicationError> {
    implementation::standard_roots(|key| enabled.contains(key))
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
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    implementation::read_entry(state, path, priority)
}

pub(crate) fn icon_cache_key(
    source: &Path,
    icon_index: i32,
    state: &mut DiscoveryState,
) -> Result<String, ApplicationError> {
    implementation::icon_cache_key(source, icon_index, state)
}

pub(crate) fn extract_icons(
    source: &Path,
    icon_index: i32,
    sizes: &[u32],
    directory: &Path,
) -> Result<(), ApplicationError> {
    implementation::extract_icons(source, icon_index, sizes, directory)
}

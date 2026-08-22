use std::path::{Path, PathBuf};

use crate::{ApplicationEntry, ApplicationError, DiscoveryState};

pub(super) fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
    Ok(Vec::new())
}

pub(super) fn is_application_path(_path: &Path) -> bool {
    false
}

pub(super) fn is_application_bundle(_path: &Path) -> bool {
    false
}

pub(super) fn read_entry(
    _state: &mut DiscoveryState,
    _path: &Path,
    _seen_at: u64,
    _priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    Ok(None)
}

pub(super) fn extract_icon(
    _source: &Path,
    _icon_index: i32,
    _size: u32,
    _target: &Path,
) -> Result<(), ApplicationError> {
    Err(ApplicationError::Configuration(
        "application discovery is unsupported on this platform".to_owned(),
    ))
}

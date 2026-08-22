use std::collections::HashMap;
#[cfg(windows)]
use std::fs::File;
use std::fs::Metadata;
#[cfg(windows)]
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::time::UNIX_EPOCH;

use crate::ApplicationError;

/// Process-local validation state owned by the discovery thread.
pub(crate) struct DiscoveryState {
    metadata: HashMap<PathBuf, Metadata>,
    #[cfg(windows)]
    executables: HashMap<PathBuf, (u64, u128, bool)>,
}

impl DiscoveryState {
    pub(crate) fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            #[cfg(windows)]
            executables: HashMap::new(),
        }
    }

    pub(crate) fn begin_scan(&mut self) {
        self.metadata.clear();
    }

    pub(crate) fn metadata(&mut self, path: &Path) -> Result<&Metadata, ApplicationError> {
        if !self.metadata.contains_key(path) {
            self.metadata.insert(path.to_path_buf(), path.metadata()?);
        }
        Ok(self.metadata.get(path).expect("metadata was inserted"))
    }

    #[cfg(windows)]
    pub(crate) fn windows_executable_stamp(
        &mut self,
        path: &Path,
    ) -> Result<Option<(u64, u128)>, ApplicationError> {
        let valid_extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("com")
            });
        if !valid_extension {
            return Ok(None);
        }
        let (length, modified) = {
            let metadata = self.metadata(path)?;
            if !metadata.is_file() {
                return Ok(None);
            }
            (
                metadata.len(),
                metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |value| value.as_nanos()),
            )
        };
        if let Some((_, _, valid)) =
            self.executables
                .get(path)
                .filter(|(cached_length, timestamp, _)| {
                    *cached_length == length && *timestamp == modified
                })
        {
            return Ok(valid.then_some((length, modified)));
        }
        let valid = validate_pe(path, length)?;
        self.executables
            .insert(path.to_path_buf(), (length, modified, valid));
        Ok(valid.then_some((length, modified)))
    }
}

#[cfg(windows)]
fn validate_pe(path: &Path, length: u64) -> Result<bool, ApplicationError> {
    if length < 68 {
        return Ok(false);
    }
    let mut file = File::open(path)?;
    let mut dos_header = [0_u8; 64];
    file.read_exact(&mut dos_header)?;
    if &dos_header[..2] != b"MZ" {
        return Ok(false);
    }
    let pe_offset = u64::from(u32::from_le_bytes(
        dos_header[60..64].try_into().unwrap_or_default(),
    ));
    if pe_offset > length.saturating_sub(4) {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(pe_offset))?;
    let mut signature = [0_u8; 4];
    file.read_exact(&mut signature)?;
    Ok(signature == *b"PE\0\0")
}

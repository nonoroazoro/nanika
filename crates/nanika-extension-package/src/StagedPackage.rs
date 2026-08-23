use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::ExtensionPackageError;

/// Immutable local copy whose digest describes the bytes used for extraction.
pub(crate) struct StagedPackage {
    path: PathBuf,
    digest: String,
}

impl StagedPackage {
    pub(crate) fn create(
        source: &Path,
        path: PathBuf,
        maximum_bytes: u64,
    ) -> Result<Self, ExtensionPackageError> {
        let result = (|| {
            let mut input = File::open(source)?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)?;
            let mut hasher = Sha256::new();
            let mut total = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                total = total.saturating_add(count as u64);
                if total > maximum_bytes {
                    return Err(ExtensionPackageError::Manifest(format!(
                        "package exceeds the {maximum_bytes} byte limit"
                    )));
                }
                hasher.update(&buffer[..count]);
                output.write_all(&buffer[..count])?;
            }
            if total == 0 {
                return Err(ExtensionPackageError::Manifest(
                    "extension package is empty".to_owned(),
                ));
            }
            output.sync_all()?;
            drop(output);

            let mut digest = String::with_capacity(64);
            for byte in hasher.finalize() {
                let _ = write!(digest, "{byte:02x}");
            }
            Ok(Self {
                path: path.clone(),
                digest,
            })
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&path);
        }
        result
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
}

impl Drop for StagedPackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

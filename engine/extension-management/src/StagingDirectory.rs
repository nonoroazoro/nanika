use std::path::{Path, PathBuf};

/// Removes an incomplete package stage unless ownership is committed.
pub(crate) struct StagingDirectory {
    path: PathBuf,
    committed: bool,
}

impl StagingDirectory {
    pub(crate) fn create(path: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            path,
            committed: false,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

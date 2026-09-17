use std::path::{Path, PathBuf};

/// Platform-resolved roots for persistent machine data and disposable caches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductPaths {
    app_data_root: PathBuf,
    cache_root: PathBuf,
}

impl ProductPaths {
    pub(crate) fn new(app_data_root: PathBuf, cache_root: PathBuf) -> Self {
        Self {
            app_data_root,
            cache_root,
        }
    }

    pub fn app_data_root(&self) -> &Path {
        &self.app_data_root
    }

    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }
}

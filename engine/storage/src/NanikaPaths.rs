use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use nanika_core::PROJECT_IDENTITY;

/// Resolved machine-local and user-configurable Nanika locations.
#[derive(Debug, Clone)]
pub struct NanikaPaths {
    app_data_root: PathBuf,
    cache_root: PathBuf,
    config_root: PathBuf,
}

impl NanikaPaths {
    pub fn discover() -> Option<Self> {
        ProjectDirs::from(
            PROJECT_IDENTITY.qualifier,
            PROJECT_IDENTITY.organization,
            PROJECT_IDENTITY.application,
        )
        .map(|dirs| Self {
            app_data_root: dirs.data_local_dir().to_path_buf(),
            cache_root: dirs.cache_dir().to_path_buf(),
            config_root: dirs.config_dir().to_path_buf(),
        })
    }

    pub fn from_roots(
        app_data_root: impl Into<PathBuf>,
        cache_root: impl Into<PathBuf>,
        config_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            app_data_root: app_data_root.into(),
            cache_root: cache_root.into(),
            config_root: config_root.into(),
        }
    }

    pub fn app_data_root(&self) -> &Path {
        &self.app_data_root
    }

    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    pub fn config_root(&self) -> &Path {
        &self.config_root
    }

    pub fn bootstrap_file(&self) -> PathBuf {
        self.app_data_root.join("bootstrap.jsonc")
    }

    pub fn database_dir(&self) -> PathBuf {
        self.app_data_root.join("databases")
    }

    pub fn host_database(&self) -> PathBuf {
        self.database_dir().join("nanika.db")
    }

    pub fn payload_dir(&self) -> PathBuf {
        self.app_data_root.join("payloads")
    }
}

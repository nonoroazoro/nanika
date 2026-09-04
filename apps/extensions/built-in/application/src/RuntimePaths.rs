use std::path::PathBuf;

use directories::ProjectDirs;
use nanika_core::PROJECT_IDENTITY;

use crate::{ApplicationError, EXTENSION_ID};

/// Resolved application extension paths, overridable for supervised launches and tests.
pub struct RuntimePaths {
    pub data_root: PathBuf,
    pub cache_root: PathBuf,
    pub config_root: PathBuf,
}

impl RuntimePaths {
    pub fn resolve(arguments: impl IntoIterator<Item = String>) -> Result<Self, ApplicationError> {
        let dirs = ProjectDirs::from(
            PROJECT_IDENTITY.qualifier,
            PROJECT_IDENTITY.organization,
            PROJECT_IDENTITY.application,
        )
        .ok_or_else(|| {
            ApplicationError::Configuration("platform data directories are unavailable".to_owned())
        })?;
        let mut paths = Self {
            data_root: dirs.data_local_dir().to_path_buf(),
            cache_root: dirs.cache_dir().to_path_buf(),
            config_root: dirs.config_dir().to_path_buf(),
        };
        for argument in arguments {
            if let Some(value) = argument.strip_prefix("--data-root=") {
                paths.data_root = absolute_path(value, "data root")?;
            } else if let Some(value) = argument.strip_prefix("--cache-root=") {
                paths.cache_root = absolute_path(value, "cache root")?;
            } else if let Some(value) = argument.strip_prefix("--config-root=") {
                paths.config_root = absolute_path(value, "config root")?;
            } else {
                return Err(ApplicationError::Configuration(format!(
                    "unsupported application extension argument: {argument}"
                )));
            }
        }
        Ok(paths)
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_root
            .join("databases/extensions")
            .join(format!("{EXTENSION_ID}.db"))
    }

    pub fn icon_root(&self) -> PathBuf {
        self.cache_root.join("icons").join(EXTENSION_ID)
    }
}

fn absolute_path(value: &str, label: &str) -> Result<PathBuf, ApplicationError> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ApplicationError::Configuration(format!(
            "application extension {label} must be absolute"
        )));
    }
    Ok(path)
}

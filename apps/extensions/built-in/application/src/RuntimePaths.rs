use std::path::PathBuf;

use crate::{ApplicationError, EXTENSION_ID};

/// Resolved application extension paths, overridable for supervised launches and tests.
pub struct RuntimePaths {
    pub data_root: PathBuf,
    pub cache_root: PathBuf,
}

impl RuntimePaths {
    pub fn resolve(arguments: impl IntoIterator<Item = String>) -> Result<Self, ApplicationError> {
        let mut data_root = None;
        let mut cache_root = None;
        for argument in arguments {
            if let Some(value) = argument.strip_prefix("--data-root=") {
                data_root = Some(absolute_path(value, "data root")?);
            } else if let Some(value) = argument.strip_prefix("--cache-root=") {
                cache_root = Some(absolute_path(value, "cache root")?);
            } else {
                return Err(ApplicationError::Configuration(format!(
                    "unsupported application extension argument: {argument}"
                )));
            }
        }
        Ok(Self {
            data_root: data_root.ok_or_else(|| {
                ApplicationError::Configuration(
                    "application extension data root is missing".to_owned(),
                )
            })?,
            cache_root: cache_root.ok_or_else(|| {
                ApplicationError::Configuration(
                    "application extension cache root is missing".to_owned(),
                )
            })?,
        })
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

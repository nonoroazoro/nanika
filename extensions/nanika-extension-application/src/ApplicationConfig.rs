use std::path::{Path, PathBuf};

use nanika_config::ConfigStore;
use serde::{Deserialize, Serialize};

use crate::ApplicationError;

const FORMAT_VERSION: u32 = 1;

/// Human-edited application discovery settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationConfig {
    pub format_version: u32,
    #[serde(default)]
    pub roots: Vec<PathBuf>,
    #[serde(default)]
    pub exclusions: Vec<PathBuf>,
}

impl ApplicationConfig {
    pub fn load(store: &ConfigStore) -> Result<Self, ApplicationError> {
        let path = Self::path(store.config_root());
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => {
                return Err(ApplicationError::Configuration(format!(
                    "application settings path is not a file: {}",
                    path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    format_version: FORMAT_VERSION,
                    ..Self::default()
                });
            }
            Err(error) => return Err(ApplicationError::Io(error)),
        }
        let config = store
            .load::<Self>(&path)
            .map_err(|error| ApplicationError::Configuration(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn path(config_root: &Path) -> PathBuf {
        config_root
            .join("extensions")
            .join(crate::EXTENSION_ID)
            .join("settings.jsonc")
    }

    pub fn validate(&self) -> Result<(), ApplicationError> {
        if self.format_version != FORMAT_VERSION {
            return Err(ApplicationError::Configuration(format!(
                "unsupported application settings format {}",
                self.format_version
            )));
        }
        for path in self.roots.iter().chain(&self.exclusions) {
            if !path.is_absolute() {
                return Err(ApplicationError::Configuration(format!(
                    "application discovery path must be absolute: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    pub fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
        crate::platform::standard_roots()
    }
}

use std::path::{Path, PathBuf};

use nanika_config::ConfigStore;
use nanika_protocol::{
    SettingControl, SettingField, SettingUpdate, SettingValue, SettingsContribution,
};
use serde::{Deserialize, Serialize};

use crate::ApplicationError;

const FORMAT_VERSION: u32 = 1;
const MAX_PATHS: usize = 256;
const MAX_PATH_BYTES: usize = 4_096;

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
        if self.roots.len() > MAX_PATHS || self.exclusions.len() > MAX_PATHS {
            return Err(ApplicationError::Configuration(format!(
                "application discovery lists exceed {MAX_PATHS} paths"
            )));
        }
        for path in self.roots.iter().chain(&self.exclusions) {
            if !path.is_absolute() || path.to_string_lossy().len() > MAX_PATH_BYTES {
                return Err(ApplicationError::Configuration(format!(
                    "application discovery path must be absolute and at most {MAX_PATH_BYTES} bytes: {}",
                    path.display()
                )));
            }
        }
        self.settings()
            .validate()
            .map_err(ApplicationError::Configuration)?;
        Ok(())
    }

    pub fn settings(&self) -> SettingsContribution {
        SettingsContribution {
            title: "Applications".to_owned(),
            fields: vec![
                path_list("roots", "Additional roots", &self.roots),
                path_list("exclusions", "Excluded paths", &self.exclusions),
            ],
        }
    }

    pub fn update(
        &self,
        store: &ConfigStore,
        updates: Vec<SettingUpdate>,
    ) -> Result<Self, ApplicationError> {
        let mut next = self.clone();
        let mut changed = Vec::with_capacity(updates.len() + 1);
        if !Self::path(store.config_root()).is_file() {
            changed.push((
                "formatVersion".to_owned(),
                serde_json::json!(FORMAT_VERSION),
            ));
        }
        let mut seen = std::collections::HashSet::with_capacity(updates.len());
        for update in updates {
            if !seen.insert(update.key.clone()) {
                return Err(ApplicationError::Configuration(format!(
                    "duplicate application setting: {}",
                    update.key
                )));
            }
            let SettingValue::StringList { values } = update.value else {
                return Err(ApplicationError::Configuration(format!(
                    "application setting has an invalid value: {}",
                    update.key
                )));
            };
            let paths = values.into_iter().map(PathBuf::from).collect::<Vec<_>>();
            let value = serde_json::to_value(&paths)
                .map_err(|error| ApplicationError::Configuration(error.to_string()))?;
            match update.key.as_str() {
                "roots" => next.roots = paths,
                "exclusions" => next.exclusions = paths,
                _ => {
                    return Err(ApplicationError::Configuration(format!(
                        "unknown application setting: {}",
                        update.key
                    )));
                }
            }
            changed.push((update.key, value));
        }
        next.validate()?;
        let path = Self::path(store.config_root());
        store
            .update::<Self>(&path, changed, |config| {
                config.validate().map_err(|error| error.to_string())
            })
            .map_err(|error| ApplicationError::Configuration(error.to_string()))
    }

    pub fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
        crate::platform::standard_roots()
    }
}

fn path_list(key: &str, title: &str, paths: &[PathBuf]) -> SettingField {
    SettingField {
        key: key.to_owned(),
        title: title.to_owned(),
        description: None,
        control: SettingControl::StringList {
            placeholder: Some("Absolute path".to_owned()),
            path: true,
            max_items: MAX_PATHS as u32,
        },
        value: SettingValue::StringList {
            values: paths
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
        },
    }
}

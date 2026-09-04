use std::collections::BTreeMap;
use std::io::ErrorKind;

use serde::{Deserialize, Serialize};

use crate::ConfigStore;

const FORMAT_VERSION: u32 = 1;

/// Synchronized enablement preferences for built-in and external extensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRegistryConfig {
    pub format_version: u32,
    #[serde(default)]
    pub extensions: BTreeMap<String, bool>,
}

impl ExtensionRegistryConfig {
    pub fn load(store: &ConfigStore) -> Result<Self, String> {
        let path = store.extensions_file();
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => {
                return Err(format!(
                    "extension registry path is not a file: {}",
                    path.display()
                ));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error.to_string()),
        }
        let config = store
            .load::<Self>(&path)
            .map_err(|error| error.to_string())?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, store: &ConfigStore) -> Result<(), String> {
        self.validate()?;
        let path = store.extensions_file();
        let current = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => Some(Self::load(store)?),
            Ok(_) => {
                return Err(format!(
                    "extension registry path is not a file: {}",
                    path.display()
                ));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        if let Some(current) = current {
            let mut updates = self
                .extensions
                .iter()
                .filter(|(extension_id, enabled)| {
                    current.extensions.get(*extension_id) != Some(*enabled)
                })
                .map(|(extension_id, enabled)| {
                    (
                        extension_id.clone(),
                        Some(serde_json::Value::Bool(*enabled)),
                    )
                })
                .collect::<Vec<_>>();
            updates.extend(
                current
                    .extensions
                    .keys()
                    .filter(|extension_id| !self.extensions.contains_key(*extension_id))
                    .map(|extension_id| (extension_id.clone(), None)),
            );
            store
                .update_object::<Self>(&path, "extensions", updates, Self::validate)
                .map(|_| ())
                .map_err(|error| error.to_string())
        } else {
            store
                .update::<Self>(
                    &path,
                    [
                        (
                            "formatVersion".to_owned(),
                            serde_json::Value::from(self.format_version),
                        ),
                        (
                            "extensions".to_owned(),
                            serde_json::to_value(&self.extensions)
                                .map_err(|error| error.to_string())?,
                        ),
                    ],
                    Self::validate,
                )
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }

    pub fn is_enabled(&self, extension_id: &str, default: bool) -> bool {
        self.extensions
            .get(extension_id)
            .copied()
            .unwrap_or(default)
    }

    pub fn set_enabled(&mut self, extension_id: impl Into<String>, enabled: bool) {
        self.extensions.insert(extension_id.into(), enabled);
    }

    pub fn remove(&mut self, extension_id: &str) {
        self.extensions.remove(extension_id);
    }

    fn validate(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "unsupported extension registry format {}",
                self.format_version
            ));
        }
        if self
            .extensions
            .keys()
            .any(|id| !nanika_core::is_valid_extension_id(id))
        {
            return Err("extension registry contains an invalid extension id".to_owned());
        }
        Ok(())
    }
}

impl Default for ExtensionRegistryConfig {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            extensions: BTreeMap::new(),
        }
    }
}

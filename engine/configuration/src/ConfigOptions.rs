use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::{CONFIG_FORMAT_VERSION, ConfigError, load_jsonc};

pub struct ConfigOptions {
    _path: PathBuf,
    _values: Map<String, Value>
}

impl ConfigOptions {
    pub(crate) fn load(path: &Path) -> Self {
        let values = match load_jsonc::<Value>(path) {
            Ok(Value::Object(values))
                if values.get("formatVersion") == Some(&Value::from(CONFIG_FORMAT_VERSION)) =>
            {
                values
            }
            Err(ConfigError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Map::new()
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "configuration could not be loaded; using defaults");
                Map::new()
            }
            Ok(_) => {
                tracing::warn!(path = %path.display(), "invalid configuration format; using defaults");
                Map::new()
            }
        };
        Self {
            _path: path.to_path_buf(),
            _values: values
        }
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get_validated(key, default, |_| Ok(()))
    }

    pub fn get_validated<T: DeserializeOwned>(
        &self,
        key: &str,
        default: T,
        validate: impl FnOnce(&T) -> Result<(), String>
    ) -> T {
        let Some(value) = self._values.get(key) else {
            return default;
        };
        match serde_json::from_value::<T>(value.clone())
            .map_err(|error| error.to_string())
            .and_then(|value| {
                validate(&value)?;
                Ok(value)
            }) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(path = %self._path.display(), %key, %error, "invalid setting; using its default");
                default
            }
        }
    }
}

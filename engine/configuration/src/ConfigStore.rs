use std::fs;
use std::path::{Path, PathBuf};

use jsonc_parser::{
    ParseOptions,
    cst::{CstInputValue, CstRootNode},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    BootstrapConfig, CONFIG_FORMAT_VERSION, ConfigError, backup_path, copy_atomic, load_jsonc,
    relative_config_path, save_jsonc, save_text_atomic, validate_bootstrap,
};

/// Store for bootstrap metadata and the effective user configuration root.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    bootstrap_path: PathBuf,
    config_root: PathBuf,
    machine_root: PathBuf,
    read_only: bool,
}

impl ConfigStore {
    /// Open the bootstrap locator, creating a valid default on first run.
    pub fn open(
        machine_root: impl AsRef<Path>,
        default_config_root: impl AsRef<Path>,
    ) -> Result<Self, ConfigError> {
        let machine_root = machine_root.as_ref();
        if !is_normalized_absolute(machine_root) {
            return Err(ConfigError::Invalid(
                "machine root must be a normalized absolute path".to_owned(),
            ));
        }
        fs::create_dir_all(machine_root)?;
        let bootstrap_path = machine_root.join("bootstrap.jsonc");
        let bootstrap_backup = backup_path(machine_root, None, &bootstrap_path)?;
        let (bootstrap, read_only, refresh_backup) = if bootstrap_path.is_file() {
            match load_jsonc::<BootstrapConfig>(&bootstrap_path).and_then(|bootstrap| {
                validate_bootstrap(&bootstrap)?;
                Ok(bootstrap)
            }) {
                Ok(bootstrap) => (bootstrap, false, true),
                Err(error) => {
                    if !bootstrap_backup.is_file() {
                        return Err(error);
                    }
                    let bootstrap = load_jsonc(&bootstrap_backup)?;
                    validate_bootstrap(&bootstrap)?;
                    (bootstrap, true, false)
                }
            }
        } else {
            let bootstrap = BootstrapConfig {
                format_version: CONFIG_FORMAT_VERSION,
                config_root: default_config_root.as_ref().to_path_buf(),
                machine_id: Uuid::new_v4(),
            };
            validate_bootstrap(&bootstrap)?;
            save_jsonc(&bootstrap_path, &bootstrap, None)?;
            (bootstrap, false, true)
        };
        fs::create_dir_all(&bootstrap.config_root)?;
        if refresh_backup {
            copy_if_changed(&bootstrap_path, &bootstrap_backup)?;
        }
        Ok(Self {
            bootstrap_path,
            config_root: bootstrap.config_root,
            machine_root: machine_root.to_path_buf(),
            read_only,
        })
    }

    pub fn bootstrap_path(&self) -> &Path {
        &self.bootstrap_path
    }

    pub fn config_root(&self) -> &Path {
        &self.config_root
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_root.join("nanika.jsonc")
    }

    pub fn extensions_file(&self) -> PathBuf {
        self.config_root.join("extensions.jsonc")
    }

    /// Parse a JSONC file into a typed Rust boundary.
    pub fn load<T: DeserializeOwned>(&self, path: impl AsRef<Path>) -> Result<T, ConfigError> {
        let path = path.as_ref();
        if path != self.bootstrap_path {
            relative_config_path(&self.config_root, path)?;
        }
        load_jsonc(path)
    }

    /// Serialize a typed value and replace a config file with a synced temporary file.
    pub fn save<T: Serialize>(&self, path: impl AsRef<Path>, value: &T) -> Result<(), ConfigError> {
        if self.read_only {
            return Err(ConfigError::Invalid(
                "configuration is read-only after recovery".to_owned(),
            ));
        }
        let path = path.as_ref();
        if path == self.bootstrap_path {
            return Err(ConfigError::Invalid(
                "bootstrap updates require the relocation boundary".to_owned(),
            ));
        }
        relative_config_path(&self.config_root, path)?;
        let backup = backup_path(&self.machine_root, Some(&self.config_root), path)?;
        save_jsonc(path, value, Some(&backup))
    }

    /// Update top-level properties while preserving unrelated JSONC comments and formatting.
    pub fn update<T: DeserializeOwned>(
        &self,
        path: impl AsRef<Path>,
        updates: impl IntoIterator<Item = (String, Value)>,
        validate: impl FnOnce(&T) -> Result<(), String>,
    ) -> Result<T, ConfigError> {
        if self.read_only {
            return Err(ConfigError::Invalid(
                "configuration is read-only after recovery".to_owned(),
            ));
        }
        let path = path.as_ref();
        if path == self.bootstrap_path {
            return Err(ConfigError::Invalid(
                "bootstrap updates require the relocation boundary".to_owned(),
            ));
        }
        relative_config_path(&self.config_root, path)?;

        let existing = if path.is_file() {
            fs::read_to_string(path)?
        } else {
            "{}\n".to_owned()
        };
        let root = CstRootNode::parse(&existing, &ParseOptions::default())
            .map_err(|error| ConfigError::Parse(error.to_string()))?;
        let object = root.object_value_or_set();
        for (key, value) in updates {
            let value = cst_value(value)?;
            if let Some(property) = object.get(&key) {
                property.set_value(value);
            } else {
                object.append(&key, value);
            }
        }
        let mut text = root.to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let typed = jsonc_parser::parse_to_serde_value(&text, &ParseOptions::default())
            .map_err(|error| ConfigError::Parse(error.to_string()))?;
        validate(&typed).map_err(ConfigError::Invalid)?;
        let backup = backup_path(&self.machine_root, Some(&self.config_root), path)?;
        save_text_atomic(path, &text, Some(&backup))?;
        Ok(typed)
    }

    /// Replace selected members of one object while preserving unrelated CST nodes.
    pub fn update_object<T: DeserializeOwned>(
        &self,
        path: impl AsRef<Path>,
        object_name: &str,
        updates: impl IntoIterator<Item = (String, Option<Value>)>,
        validate: impl FnOnce(&T) -> Result<(), String>,
    ) -> Result<T, ConfigError> {
        if self.read_only {
            return Err(ConfigError::Invalid(
                "configuration is read-only after recovery".to_owned(),
            ));
        }
        let path = path.as_ref();
        if path == self.bootstrap_path {
            return Err(ConfigError::Invalid(
                "bootstrap updates require the relocation boundary".to_owned(),
            ));
        }
        relative_config_path(&self.config_root, path)?;

        let existing = fs::read_to_string(path)?;
        let root = CstRootNode::parse(&existing, &ParseOptions::default())
            .map_err(|error| ConfigError::Parse(error.to_string()))?;
        let object = root.object_value_or_set().object_value_or_set(object_name);
        for (key, value) in updates {
            match (object.get(&key), value) {
                (Some(property), Some(value)) => property.set_value(cst_value(value)?),
                (None, Some(value)) => {
                    object.append(&key, cst_value(value)?);
                }
                (Some(property), None) => property.remove(),
                (None, None) => {}
            }
        }
        let mut text = root.to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let typed = jsonc_parser::parse_to_serde_value(&text, &ParseOptions::default())
            .map_err(|error| ConfigError::Parse(error.to_string()))?;
        validate(&typed).map_err(ConfigError::Invalid)?;
        let backup = backup_path(&self.machine_root, Some(&self.config_root), path)?;
        save_text_atomic(path, &text, Some(&backup))?;
        Ok(typed)
    }
}

fn cst_value(value: Value) -> Result<CstInputValue, ConfigError> {
    match value {
        Value::Null => Ok(CstInputValue::Null),
        Value::Bool(value) => Ok(CstInputValue::Bool(value)),
        Value::Number(value) => Ok(CstInputValue::Number(value.to_string())),
        Value::String(value) => Ok(CstInputValue::String(value)),
        Value::Array(values) => values
            .into_iter()
            .map(cst_value)
            .collect::<Result<Vec<_>, _>>()
            .map(CstInputValue::Array),
        Value::Object(values) => values
            .into_iter()
            .map(|(key, value)| Ok((key, cst_value(value)?)))
            .collect::<Result<Vec<_>, ConfigError>>()
            .map(CstInputValue::Object),
    }
}

fn is_normalized_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::Normal(_)
            )
        })
}

fn copy_if_changed(source: &Path, target: &Path) -> Result<(), ConfigError> {
    if target.is_file() && fs::read(source)? == fs::read(target)? {
        return Ok(());
    }
    copy_atomic(source, target)
}

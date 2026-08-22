use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::{
    BootstrapConfig, CONFIG_FORMAT_VERSION, ConfigError, backup_path, copy_atomic, load_jsonc,
    relative_config_path, save_jsonc, validate_bootstrap,
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

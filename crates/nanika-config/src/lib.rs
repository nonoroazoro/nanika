//! Human-edited JSONC configuration boundary.

#![allow(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use jsonc_parser::{ParseOptions, errors::ParseError, parse_to_serde_value};
use nanika_storage::NanikaPaths;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

pub const CONFIG_FORMAT_VERSION: u32 = 1;

/// Machine-local locator for the relocatable synchronized configuration tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapConfig {
    pub format_version: u32,
    pub config_root: PathBuf,
    pub machine_id: Uuid,
}

/// Configuration boundary errors.
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(String),
    Serialize(serde_json::Error),
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "configuration I/O error: {error}"),
            Self::Parse(error) => write!(formatter, "configuration parse error: {error}"),
            Self::Serialize(error) => {
                write!(formatter, "configuration serialization error: {error}")
            }
            Self::Invalid(error) => write!(formatter, "invalid configuration: {error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

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
    pub fn open(paths: &NanikaPaths) -> Result<Self, ConfigError> {
        paths.ensure_machine_local_dirs()?;
        let bootstrap_path = paths.bootstrap_file();
        let (bootstrap, read_only) = if bootstrap_path.is_file() {
            match load_jsonc::<BootstrapConfig>(&bootstrap_path) {
                Ok(bootstrap) => (bootstrap, false),
                Err(error) => {
                    let backup = backup_path(paths.app_data_root(), &bootstrap_path)?;
                    if !backup.is_file() {
                        return Err(error);
                    }
                    (load_jsonc(&backup)?, true)
                }
            }
        } else {
            let bootstrap = BootstrapConfig {
                format_version: CONFIG_FORMAT_VERSION,
                config_root: paths.config_root().to_path_buf(),
                machine_id: Uuid::new_v4(),
            };
            save_jsonc(&bootstrap_path, &bootstrap, None)?;
            (bootstrap, false)
        };
        validate_bootstrap(&bootstrap)?;
        fs::create_dir_all(&bootstrap.config_root)?;
        Ok(Self {
            bootstrap_path,
            config_root: bootstrap.config_root,
            machine_root: paths.app_data_root().to_path_buf(),
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
        let backup = backup_path(&self.machine_root, path)?;
        save_jsonc(path, value, Some(&backup))
    }
}

fn load_jsonc<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, ConfigError> {
    let text = fs::read_to_string(path)?;
    parse_to_serde_value(&text, &ParseOptions::default()).map_err(parse_error)
}

fn save_jsonc<T: Serialize>(
    path: impl AsRef<Path>,
    value: &T,
    backup: Option<&Path>,
) -> Result<(), ConfigError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| ConfigError::Invalid(error.to_string()))?
            .as_nanos()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    if let Some(backup) = backup
        && path.is_file()
    {
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, backup)?;
    }
    atomic_replace(&temporary, path)?;
    Ok(())
}

fn backup_path(machine_root: &Path, path: &Path) -> Result<PathBuf, ConfigError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| ConfigError::Invalid("configuration path has no file name".to_owned()))?;
    Ok(machine_root.join("backups").join("config").join(file_name))
}

fn parse_error(error: ParseError) -> ConfigError {
    ConfigError::Parse(error.to_string())
}

fn validate_bootstrap(config: &BootstrapConfig) -> Result<(), ConfigError> {
    if config.format_version != CONFIG_FORMAT_VERSION {
        return Err(ConfigError::Invalid(format!(
            "unsupported bootstrap format version {}",
            config.format_version
        )));
    }
    if config.config_root.as_os_str().is_empty() {
        return Err(ConfigError::Invalid("config root is empty".to_owned()));
    }
    Ok(())
}

#[cfg(not(windows))]
fn atomic_replace(temporary: &Path, target: &Path) -> io::Result<()> {
    fs::rename(temporary, target)
}

#[cfg(windows)]
fn atomic_replace(temporary: &Path, target: &Path) -> io::Result<()> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temporary: Vec<u16> = temporary.as_os_str().encode_wide().chain(once(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(once(0)).collect();
    if unsafe {
        MoveFileExW(
            temporary.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(io::Error::from_raw_os_error(
            unsafe { GetLastError() } as i32
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BootstrapConfig, CONFIG_FORMAT_VERSION, ConfigStore};
    use nanika_storage::NanikaPaths;
    use uuid::Uuid;

    #[test]
    fn bootstrap_is_created_and_reused() {
        let root = std::env::temp_dir().join(format!("nanika-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = NanikaPaths::from_roots(&root, root.join("cache"));
        let first = ConfigStore::open(&paths).expect("store should open");
        let bootstrap: BootstrapConfig = first.load(first.bootstrap_path()).expect("bootstrap");
        assert_eq!(bootstrap.format_version, CONFIG_FORMAT_VERSION);
        let second = ConfigStore::open(&paths).expect("store should reopen");
        let same: BootstrapConfig = second.load(second.bootstrap_path()).expect("bootstrap");
        assert_eq!(same.machine_id, bootstrap.machine_id);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn comments_are_accepted_at_the_typed_boundary() {
        let root = std::env::temp_dir().join(format!("nanika-config-jsonc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = NanikaPaths::from_roots(&root, root.join("cache"));
        let store = ConfigStore::open(&paths).expect("store should open");
        let file = store.config_file();
        std::fs::write(
            &file,
            r#"{
              // keep this comment
              "formatVersion": 1,
              "configRoot": "config",
              "machineId": "00000000-0000-0000-0000-000000000000"
            }"#,
        )
        .expect("write JSONC");
        let value: BootstrapConfig = store.load(&file).expect("JSONC should parse");
        assert_eq!(value.machine_id, Uuid::nil());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_bootstrap_recovers_last_known_good_and_becomes_read_only() {
        let root =
            std::env::temp_dir().join(format!("nanika-config-recovery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = NanikaPaths::from_roots(&root, root.join("cache"));
        let store = ConfigStore::open(&paths).expect("store should open");
        let bootstrap: BootstrapConfig = store.load(store.bootstrap_path()).expect("bootstrap");
        store
            .save(store.bootstrap_path(), &bootstrap)
            .expect("bootstrap backup should be written");
        std::fs::write(store.bootstrap_path(), "{ malformed").expect("corrupt bootstrap");
        let recovered = ConfigStore::open(&paths).expect("backup should recover");
        assert!(recovered.is_read_only());
        assert!(recovered.save(recovered.config_file(), &bootstrap).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}

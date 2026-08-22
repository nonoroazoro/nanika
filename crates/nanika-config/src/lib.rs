//! Human-edited JSONC configuration boundary.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use jsonc_parser::{ParseOptions, errors::ParseError, parse_to_serde_value};
use serde::{Serialize, de::DeserializeOwned};

#[path = "BootstrapConfig.rs"]
mod bootstrap_config;
#[path = "ConfigError.rs"]
mod config_error;
#[path = "ConfigStore.rs"]
mod config_store;

pub use bootstrap_config::*;
pub use config_error::*;
pub use config_store::*;

pub const CONFIG_FORMAT_VERSION: u32 = 1;

pub(crate) fn load_jsonc<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, ConfigError> {
    let text = fs::read_to_string(path)?;
    parse_to_serde_value(&text, &ParseOptions::default()).map_err(parse_error)
}

pub(crate) fn save_jsonc<T: Serialize>(
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
    let result = (|| {
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
            copy_atomic(path, backup)?;
        }
        atomic_replace(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn backup_path(
    machine_root: &Path,
    config_root: Option<&Path>,
    path: &Path,
) -> Result<PathBuf, ConfigError> {
    let relative = config_root
        .and_then(|root| relative_config_path(root, path).ok())
        .map(Path::to_path_buf)
        .or_else(|| path.file_name().map(PathBuf::from))
        .ok_or_else(|| ConfigError::Invalid("configuration path has no file name".to_owned()))?;
    Ok(machine_root.join("backups").join("config").join(relative))
}

fn relative_config_path<'a>(root: &Path, path: &'a Path) -> Result<&'a Path, ConfigError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        ConfigError::Invalid("configuration path is outside the config root".to_owned())
    })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(ConfigError::Invalid(
            "configuration path is not a normalized file path".to_owned(),
        ));
    }
    Ok(relative)
}

fn copy_atomic(source: &Path, target: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = target.with_extension(format!(
        "backup-tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| ConfigError::Invalid(error.to_string()))?
            .as_nanos()
    ));
    let result = (|| {
        let mut input = File::open(source)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        drop(output);
        atomic_replace(&temporary, target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
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
    if !config.config_root.is_absolute()
        || config.config_root.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err(ConfigError::Invalid(
            "config root must be a normalized absolute path".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn atomic_replace(temporary: &Path, target: &Path) -> io::Result<()> {
    fs::rename(temporary, target)
}

#[cfg(windows)]
#[allow(unsafe_code)]
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

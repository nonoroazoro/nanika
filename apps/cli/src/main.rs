//! Nanika extension management command line.

#[path = "Command.rs"]
mod command;

use std::process::ExitCode;
use std::{ffi::OsStr, fs, io::Write, path::Path};

use command::Command;
use nanika_config::ConfigStore;
use nanika_extension_package::{
    install_package, remove_extension, set_extension_enabled, update_package,
};
use nanika_platform::{InstanceRole, open_regular_file};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Nanika: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let command = Command::parse(std::env::args().skip(1))?;
    let paths = nanika_storage::NanikaPaths::discover()
        .ok_or_else(|| "failed to resolve platform data directories".to_owned())?;
    if let Command::Diagnostics(destination) = &command {
        export_diagnostics(paths.app_data_root(), destination)?;
        return Ok(format!("exported diagnostics to {}", destination.display()));
    }
    let instance = nanika_platform::acquire_instance(
        nanika_foundation::PROJECT_IDENTITY.bundle_id,
        paths.app_data_root(),
    )
    .map_err(|error| error.to_string())?;
    let InstanceRole::Primary(_instance) = instance else {
        return Err("close the running Nanika host before changing extensions".to_owned());
    };
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root())
        .map_err(|error| error.to_string())?;
    match command {
        Command::Install(package) => {
            let installed =
                install_package(&package, &paths, &store).map_err(|error| error.to_string())?;
            Ok(format!(
                "installed {} from {}",
                installed.extension_id,
                package.display()
            ))
        }
        Command::Update(package) => {
            let installed =
                update_package(&package, &paths, &store).map_err(|error| error.to_string())?;
            Ok(format!(
                "updated {} from {}",
                installed.extension_id,
                package.display()
            ))
        }
        Command::Enable(extension_id) => {
            set_extension_enabled(&extension_id, true, &paths, &store)
                .map_err(|error| error.to_string())?;
            Ok(format!("enabled {extension_id}"))
        }
        Command::Disable(extension_id) => {
            set_extension_enabled(&extension_id, false, &paths, &store)
                .map_err(|error| error.to_string())?;
            Ok(format!("disabled {extension_id}"))
        }
        Command::Remove(extension_id) => {
            remove_extension(&extension_id, &paths, &store).map_err(|error| error.to_string())?;
            Ok(format!("removed {extension_id}"))
        }
        Command::Diagnostics(_) => unreachable!("diagnostics returns before the mutation gate"),
    }
}

fn export_diagnostics(app_data_root: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err("diagnostic destination already exists".to_owned());
    }
    if let Some(parent) = destination.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file_name = destination
        .file_name()
        .ok_or_else(|| "diagnostic destination must include a file name".to_owned())?;
    let temporary = destination.with_file_name(format!(
        ".{}.{}-{}.partial",
        file_name.to_string_lossy(),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    ));
    let result = (|| {
        let file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        archive
            .start_file("diagnostics.txt", options)
            .map_err(|error| error.to_string())?;
        writeln!(archive, "Nanika version: {}", env!("CARGO_PKG_VERSION"))
            .and_then(|()| {
                writeln!(
                    archive,
                    "Platform: {}-{}",
                    nanika_platform::target_platform(),
                    std::env::consts::ARCH
                )
            })
            .map_err(|error| error.to_string())?;

        let log_root = app_data_root.join("logs");
        let log_root_metadata = fs::symlink_metadata(&log_root);
        if log_root_metadata
            .as_ref()
            .is_ok_and(|metadata| metadata.file_type().is_dir())
        {
            let mut logs = Vec::new();
            for entry in fs::read_dir(log_root).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                let file_type = entry.file_type().map_err(|error| error.to_string())?;
                if file_type.is_file() && is_nanika_log_name(&entry.file_name()) {
                    logs.push(entry);
                }
            }
            logs.sort_by_key(|entry| entry.file_name());
            for entry in logs {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                archive
                    .start_file(format!("logs/{name}"), options)
                    .map_err(|error| error.to_string())?;
                let mut input =
                    open_regular_file(&entry.path()).map_err(|error| error.to_string())?;
                std::io::copy(&mut input, &mut archive).map_err(|error| error.to_string())?;
            }
        } else if let Err(error) = log_root_metadata
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.to_string());
        }
        archive
            .finish()
            .map_err(|error| error.to_string())?
            .sync_all()
            .map_err(|error| error.to_string())?;
        fs::hard_link(&temporary, destination).map_err(|error| error.to_string())?;
        fs::remove_file(&temporary).map_err(|error| error.to_string())?;
        Ok(())
    })();
    if result.is_err()
        && let Err(cleanup_error) = fs::remove_file(&temporary)
        && cleanup_error.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!(
            "Nanika: failed to remove incomplete diagnostics file {}: {cleanup_error}",
            temporary.display()
        );
    }
    result
}

fn is_nanika_log_name(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(date) = name
        .strip_prefix("nanika.")
        .and_then(|name| name.strip_suffix(".log"))
    else {
        return false;
    };
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

#[cfg(test)]
#[path = "../tests/main.rs"]
mod tests;

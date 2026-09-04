//! Nanika extension management command line.

#[path = "Command.rs"]
mod command;

use std::process::ExitCode;
use std::{
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::Path,
};

use command::Command;
use nanika_config::ConfigStore;
use nanika_extension_package::{
    install_package, remove_extension, set_extension_enabled, update_package,
};
use nanika_platform::InstanceRole;

const MAX_DIAGNOSTIC_LOG_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DIAGNOSTIC_LOG_FILES: usize = 8;

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
        nanika_core::PROJECT_IDENTITY.bundle_id,
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
                    std::env::consts::OS,
                    std::env::consts::ARCH
                )
            })
            .map_err(|error| error.to_string())?;

        let log_root = app_data_root.join("logs");
        let mut total_bytes = 0_u64;
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
            for entry in logs.into_iter().rev().take(MAX_DIAGNOSTIC_LOG_FILES).rev() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                archive
                    .start_file(format!("logs/{name}"), options)
                    .map_err(|error| error.to_string())?;
                let input = open_regular_file(&entry.path()).map_err(|error| error.to_string())?;
                let remaining = MAX_DIAGNOSTIC_LOG_BYTES.saturating_sub(total_bytes);
                let copied = copy_bounded(input, &mut archive, remaining)
                    .map_err(|error| error.to_string())?;
                total_bytes = total_bytes.saturating_add(copied);
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
        let _ = fs::remove_file(&temporary);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
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

fn copy_bounded(
    input: impl Read,
    output: &mut impl Write,
    maximum_bytes: u64,
) -> std::io::Result<u64> {
    let copied = std::io::copy(&mut input.take(maximum_bytes.saturating_add(1)), output)?;
    if copied > maximum_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "diagnostic logs exceed the 32 MiB export limit",
        ));
    }
    Ok(copied)
}

#[cfg(unix)]
fn open_regular_file(path: &Path) -> std::io::Result<fs::File> {
    let before = fs::symlink_metadata(path)?;
    if !before.file_type().is_file() || before.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "diagnostic log is not a regular file",
        ));
    }
    let file = fs::File::open(path)?;
    let after = file.metadata()?;
    if !same_file_identity(&before, &after) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "diagnostic log changed while it was opened",
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn same_file_identity(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;

    before.dev() == after.dev() && before.ino() == after.ino()
}

#[cfg(windows)]
fn open_regular_file(path: &Path) -> std::io::Result<fs::File> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "diagnostic log is not a regular file",
        ));
    }
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
fn open_regular_file(_path: &Path) -> std::io::Result<fs::File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "diagnostic log validation is unsupported",
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use super::export_diagnostics;

    #[test]
    fn diagnostics_export_contains_metadata_and_logs() {
        let root = std::env::temp_dir().join(format!(
            "nanika-cli-diagnostics-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos()
        ));
        let logs = root.join("logs");
        std::fs::create_dir_all(&logs).expect("log directory should be created");
        std::fs::write(logs.join("nanika.2026-08-23.log"), "host starting\n")
            .expect("fixture log should be written");
        std::fs::write(logs.join("private.txt"), "must not be exported\n")
            .expect("stray file should be written");
        let destination = root.join("diagnostics.zip");

        export_diagnostics(&root, &destination).expect("diagnostics should export");

        let file = std::fs::File::open(&destination).expect("archive should exist");
        let mut archive = zip::ZipArchive::new(file).expect("archive should open");
        let mut metadata = String::new();
        archive
            .by_name("diagnostics.txt")
            .expect("metadata should exist")
            .read_to_string(&mut metadata)
            .expect("metadata should be readable");
        assert!(metadata.contains("Nanika version:"));
        assert!(archive.by_name("logs/nanika.2026-08-23.log").is_ok());
        assert!(archive.by_name("logs/private.txt").is_err());

        std::fs::remove_dir_all(root).expect("test directory should be removed");
    }

    #[test]
    fn diagnostics_never_overwrites_the_destination() {
        let root = temporary_root("no-clobber");
        std::fs::create_dir_all(root.join("logs")).expect("log directory should exist");
        let destination = root.join("diagnostics.zip");
        std::fs::write(&destination, "keep").expect("destination fixture should exist");

        assert!(export_diagnostics(&root, &destination).is_err());
        assert_eq!(
            std::fs::read_to_string(&destination).expect("destination should remain readable"),
            "keep"
        );
        std::fs::remove_dir_all(root).expect("test directory should be removed");
    }

    #[test]
    fn bounded_copy_rejects_growth_beyond_the_limit() {
        let mut output = Vec::new();
        let error = super::copy_bounded(&b"12345"[..], &mut output, 4)
            .expect_err("oversized input should fail");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    fn temporary_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nanika-cli-diagnostics-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos()
        ))
    }
}

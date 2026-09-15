use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Replace the destination with a completed temporary file without a copy fallback.
pub fn atomic_replace(temporary: &Path, target: &Path) -> io::Result<()> {
    fs::rename(temporary, target)
}

/// Apply the package executable mode after the caller validates the entrypoint.
pub fn make_executable(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

/// Locate an inventory-owned executable beside the current host executable.
pub fn companion_executable(current_executable: &Path, binary_name: &str) -> PathBuf {
    current_executable.with_file_name(binary_name)
}

/// Open a diagnostic file while rejecting non-files and detected link substitution.
pub fn open_regular_file(path: &Path) -> std::io::Result<fs::File> {
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

fn same_file_identity(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;

    before.dev() == after.dev() && before.ino() == after.ino()
}

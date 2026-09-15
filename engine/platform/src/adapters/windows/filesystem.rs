use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Replace the destination with a completed temporary file without a copy fallback.
pub fn atomic_replace(temporary: &Path, target: &Path) -> io::Result<()> {
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

/// Windows executables do not require a Unix permission change.
pub fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Locate an inventory-owned executable beside the current host executable.
pub fn companion_executable(current_executable: &Path, binary_name: &str) -> PathBuf {
    current_executable.with_file_name(format!("{binary_name}.exe"))
}

/// Open a diagnostic file while rejecting non-files and detected link substitution.
pub fn open_regular_file(path: &Path) -> std::io::Result<fs::File> {
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

/// Retrieve a square RGBA system icon for an existing file or directory.
/// Call on a blocking worker, never on a UI event loop.
pub fn file_icon_pixels(
    path: &std::path::Path,
    icon_index: i32,
    size: u32,
) -> std::io::Result<Vec<u8>> {
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().contains(&0)
        || !(1..=512).contains(&size)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid file icon path or size",
        ));
    }
    path.metadata()?;
    crate::adapter::file_icon::pixels(path, icon_index, size)
}

/// Retrieve the native Shell icon used to represent an existing file or directory.
///
/// Windows deliberately uses the system image list here instead of the image
/// factory used by application discovery. Some file associations return an
/// oversized bitmap canvas from `IShellItemImageFactory`, which makes the real
/// icon unreadably small when rendered by Clipboard.
pub fn shell_file_icon_pixels(path: &std::path::Path, size: u32) -> std::io::Result<Vec<u8>> {
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().contains(&0)
        || !(1..=512).contains(&size)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid Shell file icon path or size",
        ));
    }
    path.metadata()?;
    #[cfg(target_os = "windows")]
    {
        crate::adapter::file_icon::shell_pixels(path, size)
    }
    #[cfg(target_os = "macos")]
    {
        crate::adapter::file_icon::pixels(path, 0, size)
    }
}

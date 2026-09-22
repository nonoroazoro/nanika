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

/// Retrieve a square native preview up to 512 px, falling back to the file icon.
/// Call on a blocking worker; native providers may read the source.
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
    crate::adapter::file_icon::shell_pixels(path, size)
}

/// List artwork is independent of content previews on both supported platforms.
pub(crate) fn cached_list_pixels(
    path: &std::path::Path,
    preview: &[u8],
) -> std::io::Result<Vec<u8>> {
    let _ = preview;
    crate::adapter::file_icon::list_pixels(path, 128)
}

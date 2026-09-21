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

/// Retrieve bounded native artwork for an existing file or directory.
///
/// Windows requests a fitted Shell thumbnail. macOS uses Image I/O for images and
/// Quick Look for other previewable content. Both adapters fall back to native file icons.
/// Call on a blocking worker. The returned square RGBA buffer is bounded by the requested size,
/// at most 512 pixels.
/// Native thumbnail providers may read the source to generate missing thumbnails.
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

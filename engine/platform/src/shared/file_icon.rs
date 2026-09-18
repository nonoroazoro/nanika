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
/// Windows requests a fitted Shell thumbnail, with native icon fallbacks;
/// macOS uses its native file icon. Call on a blocking worker. The returned
/// square RGBA buffer is bounded by the requested size, at most 512 pixels.
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
    #[cfg(target_os = "windows")]
    {
        crate::adapter::file_icon::shell_pixels(path, size)
    }
    #[cfg(target_os = "macos")]
    {
        crate::adapter::file_icon::pixels(path, 0, size)
    }
}

/// List artwork is independent of content previews on Windows. macOS retains
/// its existing downsampled NSWorkspace icon without a second native request.
pub(crate) fn cached_list_pixels(
    path: &std::path::Path,
    preview: &[u8],
) -> std::io::Result<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        let _ = preview;
        crate::adapter::file_icon::list_pixels(path, 128)
    }
    #[cfg(target_os = "macos")]
    {
        let _ = path;
        crate::normalize_icon_rgba(preview, 512, 512, 128)
            .ok_or_else(|| std::io::Error::other("system file icon is empty"))
    }
}

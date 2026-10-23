use nanika_protocol::ClipboardContent;

pub(crate) fn content_kind(content_type: &str) -> Result<Option<&'static str>, String> {
    match content_type {
        "all" => Ok(None),
        "text" => Ok(Some("text")),
        "files" => Ok(Some("files")),
        "images" => Ok(Some("image")),
        _ => Err("invalid clipboard content type".into())
    }
}

pub(crate) fn matches(
    query: &str,
    content: &ClipboardContent,
    dimensions: Option<crate::ClipboardImageDimensions>
) -> bool {
    query.is_empty()
        || match content {
            ClipboardContent::Text { value } => value.to_lowercase().contains(query),
            ClipboardContent::Files { paths } => {
                paths.iter().any(|path| path.to_lowercase().contains(query))
            }
            ClipboardContent::PngFile { .. } => {
                // Domain search terms stay independent of the current display language.
                ["image", "图片"].iter().any(|term| term.contains(query))
                    || dimensions
                        .is_some_and(|d| format!("{} x {}", d.width, d.height).contains(query))
            }
        }
}

use nanika_protocol::ClipboardContent;

pub(crate) fn content_kind(content_type: &str) -> Result<Option<&'static str>, String> {
    match content_type {
        "all" => Ok(None),
        "text" => Ok(Some("text")),
        "files" => Ok(Some("files")),
        "images" => Ok(Some("image")),
        _ => Err("invalid clipboard content type".into()),
    }
}

pub(crate) fn matches(query: &str, title: &str, content: &ClipboardContent) -> bool {
    query.is_empty()
        || title.to_lowercase().contains(query)
        || match content {
            ClipboardContent::Text { value } => value.to_lowercase().contains(query),
            ClipboardContent::Files { paths } => {
                paths.iter().any(|path| path.to_lowercase().contains(query))
            }
            ClipboardContent::PngFile { .. } => false,
        }
}

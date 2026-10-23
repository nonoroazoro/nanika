use nanika_protocol::ClipboardContent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardEntry {
    pub entry_id: String,
    pub image_dimensions: Option<crate::ClipboardImageDimensions>,
    pub content: ClipboardContent,
    pub byte_size: u64,
    pub captured_at: u64
}

impl ClipboardEntry {
    pub(crate) fn summary(&self) -> Result<crate::ClipboardSummary, String> {
        use crate::ClipboardSummary;
        match (&self.content, self.image_dimensions) {
            (ClipboardContent::Text { value }, None) => Ok(ClipboardSummary::Text {
                preview: crate::labels::text_preview(value)
            }),
            (ClipboardContent::Files { paths }, None) => Ok(ClipboardSummary::Files {
                first_path: paths.first().ok_or("clipboard file list is empty")?.clone(),
                count: u32::try_from(paths.len()).map_err(|e| e.to_string())?
            }),
            (ClipboardContent::PngFile { .. }, Some(dimensions))
                if nanika_protocol::png_dimensions_within_limits(
                    dimensions.width,
                    dimensions.height
                ) =>
            {
                Ok(ClipboardSummary::Image { dimensions })
            }
            _ => Err("clipboard image dimensions do not match the content".into())
        }
    }
}

use std::path::Path;

use nanika_protocol::{LocalizedText, message};

/// Lightweight row facts. Original text and file paths remain in the copy payload.
#[derive(Debug, Clone)]
pub(crate) enum ClipboardSummary {
    Text {
        preview: String
    },
    Files {
        first_path: String,
        count: u32
    },
    Image {
        dimensions: crate::ClipboardImageDimensions
    }
}

impl ClipboardSummary {
    pub fn title(&self) -> Result<LocalizedText, String> {
        Ok(match self {
            Self::Text { preview } => preview.clone().into(),
            Self::Files { first_path, count } => {
                let name = Path::new(first_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(first_path);
                // Leave space for the count, translated phrase and Unicode isolation.
                let name = crate::labels::display_label(name, 96);
                match count {
                    0 => return Err("clipboard file list is empty".into()),
                    1 => name.into(),
                    _ => message(
                        "entry-files",
                        [
                            ("name".into(), name.into()),
                            ("remaining".into(), i64::from(count - 1).into())
                        ]
                    )
                }
            }
            Self::Image { dimensions } => message(
                "entry-image",
                [
                    ("width".into(), i64::from(dimensions.width).into()),
                    ("height".into(), i64::from(dimensions.height).into())
                ]
            )
        })
    }
}

#[cfg(test)]
#[path = "../tests/ClipboardSummary.rs"]
mod tests;

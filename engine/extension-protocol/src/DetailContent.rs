use serde::{Deserialize, Serialize};

use crate::LocalizedText;

use crate::ImageSource;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DetailContent {
    Text {
        value: String,
        document: crate::TextDocument,
        /// Presentation is optional; text remains available independently.
        syntax_highlighting: bool
    },
    Files {
        files: Vec<crate::ViewFile>
    },
    Image {
        source: ImageSource,
        alternative_text: LocalizedText
    }
}

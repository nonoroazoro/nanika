use serde::{Deserialize, Serialize};

use crate::ImageSource;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DetailContent {
    Text {
        value: String,
        /// Immutable content identity, independent of the enclosing view revision.
        text_id: String,
        /// Zero-based chunk position. A read returns this chunk only, never a growing prefix.
        chunk_index: usize,
        total_chunks: usize,
    },
    Files {
        files: Vec<crate::ViewFile>,
    },
    Image {
        source: ImageSource,
        alternative_text: String,
    },
}

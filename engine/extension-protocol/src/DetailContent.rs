use serde::{Deserialize, Serialize};

use crate::ImageSource;

/// Bounded semantic content rendered by the shared detail surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DetailContent {
    Text {
        value: String,
    },
    Files {
        names: Vec<String>,
    },
    Image {
        source: ImageSource,
        alternative_text: String,
    },
}

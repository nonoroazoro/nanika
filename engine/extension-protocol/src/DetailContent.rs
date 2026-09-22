use serde::{Deserialize, Serialize};

use crate::ImageSource;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DetailContent {
    Text {
        value: String,
    },
    Files {
        files: Vec<crate::ViewFile>,
    },
    Image {
        source: ImageSource,
        alternative_text: String,
    },
}

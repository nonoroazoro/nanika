use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClipboardContent {
    Text { value: String },
    Files { paths: Vec<String> },
    PngFile { path: String },
}

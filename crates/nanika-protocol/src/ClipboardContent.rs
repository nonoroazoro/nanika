use serde::{Deserialize, Serialize};

/// Clipboard payload accepted by the host clipboard service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClipboardContent {
    Text { value: String },
    Files { paths: Vec<String> },
    PngFile { path: String },
}

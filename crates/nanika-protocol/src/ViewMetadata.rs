use serde::{Deserialize, Serialize};

/// One title and value pair in a detail surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewMetadata {
    pub title: String,
    pub value: String,
}

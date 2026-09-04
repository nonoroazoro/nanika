use serde::{Deserialize, Serialize};

use crate::{ViewAction, ViewMetadata};

/// Text content and structured metadata rendered as a detail surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailView {
    pub title: Option<String>,
    pub body: String,
    pub metadata: Vec<ViewMetadata>,
    pub actions: Vec<ViewAction>,
}

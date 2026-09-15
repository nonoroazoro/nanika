use serde::{Deserialize, Serialize};

use crate::{DetailContent, ViewAction, ViewMetadata};

/// Semantic content and structured metadata rendered as a detail surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailView {
    pub title: Option<String>,
    pub content: DetailContent,
    pub metadata: Vec<ViewMetadata>,
    pub actions: Vec<ViewAction>,
}

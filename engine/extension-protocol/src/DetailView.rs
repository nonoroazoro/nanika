use serde::{Deserialize, Serialize};

use crate::{Action, DetailContent, ViewMetadata};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailView {
    pub title: Option<String>,
    pub content: DetailContent,
    pub metadata: Vec<ViewMetadata>,
    pub actions: Vec<Action>,
}

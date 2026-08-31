use serde::{Deserialize, Serialize};

/// Semantic layout selected by an extension for a list view.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ListLayout {
    Plain,
    Split,
}

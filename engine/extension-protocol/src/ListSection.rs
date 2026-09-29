use serde::{Deserialize, Serialize};

use crate::ListItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSection {
    pub id: String,
    pub title: Option<String>,
    /// Zero-based position of the first delivered item within this section.
    pub offset: usize,
    /// Exact number of matches in the full section, including undelivered rows.
    pub total: usize,
    pub items: Vec<ListItem>,
}

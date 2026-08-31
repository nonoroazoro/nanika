use serde::{Deserialize, Serialize};

use crate::ListItem;

/// A labeled group of items in a list view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSection {
    pub id: String,
    pub title: Option<String>,
    pub items: Vec<ListItem>,
}

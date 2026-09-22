use serde::{Deserialize, Serialize};

use crate::ListItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSection {
    pub id: String,
    pub title: Option<String>,
    pub items: Vec<ListItem>,
}

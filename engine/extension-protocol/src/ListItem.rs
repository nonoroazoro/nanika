use serde::{Deserialize, Serialize};

use crate::ViewAction;

/// One selectable item in a host-rendered list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub actions: Vec<ViewAction>,
}

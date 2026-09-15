use serde::{Deserialize, Serialize};

use crate::{ViewAction, ViewItemIcon};

/// One selectable item in a host-rendered list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<ViewItemIcon>,
    pub actions: Vec<ViewAction>,
}

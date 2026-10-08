use serde::{Deserialize, Serialize};

use crate::{Action, ViewItemIcon};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<ViewItemIcon>,
    pub actions: Vec<Action>,
}

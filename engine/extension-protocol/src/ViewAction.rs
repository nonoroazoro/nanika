use serde::{Deserialize, Serialize};

use crate::ViewActionStyle;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewAction {
    pub id: String,
    pub title: String,
    /// Replacement label that requires a second click before invoking a destructive action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_title: Option<String>,
    pub style: ViewActionStyle,
}

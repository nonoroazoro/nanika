use serde::{Deserialize, Serialize};

use crate::ViewActionStyle;

/// One semantic action exposed by an extension view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewAction {
    pub id: String,
    pub title: String,
    pub style: ViewActionStyle,
}

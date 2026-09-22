use serde::{Deserialize, Serialize};

use crate::ViewFilterOption;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewFilter {
    pub id: String,
    pub selected_value: String,
    pub options: Vec<ViewFilterOption>,
}

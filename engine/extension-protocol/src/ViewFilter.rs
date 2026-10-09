use serde::{Deserialize, Serialize};

use crate::ViewFilterOption;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewFilter {
    pub id: String,
    pub selected_value: String,
    pub options: Vec<ViewFilterOption>
}

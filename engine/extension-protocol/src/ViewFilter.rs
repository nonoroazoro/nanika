use serde::{Deserialize, Serialize};

use crate::ViewFilterOption;

/// One host-rendered filter control associated with a list view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewFilter {
    pub id: String,
    pub selected_value: String,
    pub options: Vec<ViewFilterOption>,
}

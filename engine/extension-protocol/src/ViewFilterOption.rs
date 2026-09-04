use serde::{Deserialize, Serialize};

/// One selectable value in a view filter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewFilterOption {
    pub value: String,
    pub title: String,
}

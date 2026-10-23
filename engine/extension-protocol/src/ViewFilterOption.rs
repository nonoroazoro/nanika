use serde::{Deserialize, Serialize};

use crate::LocalizedText;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewFilterOption {
    pub value: String,
    pub title: LocalizedText
}

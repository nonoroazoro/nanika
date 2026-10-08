use serde::{Deserialize, Serialize};

/// Selection remains authoritative when its row is outside the delivered window.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSelection {
    pub index: usize,
    pub item: crate::ListItem,
}

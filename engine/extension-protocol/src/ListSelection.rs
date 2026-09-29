use serde::{Deserialize, Serialize};

/// Selection remains authoritative when its row is outside the delivered window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSelection {
    pub index: usize,
    pub item: crate::ListItem,
}

use crate::StoredUsage;

/// Search state loaded before the search owner starts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchStorageState {
    pub input_history: Vec<String>,
    pub usage: Vec<StoredUsage>,
}

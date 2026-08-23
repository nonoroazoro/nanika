use serde::{Deserialize, Serialize};

/// A bounded searchable result contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub entry_id: String,
    pub title: String,
    pub action_id: String,
    pub aliases: Vec<String>,
}

use serde::{Deserialize, Serialize};

use crate::IconReference;

/// A bounded searchable result contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub entry_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub action_id: String,
    pub aliases: Vec<String>,
    pub icon: Option<IconReference>,
}

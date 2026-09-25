use serde::{Deserialize, Serialize};

use crate::{CandidateKind, IconSource};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub kind: CandidateKind,
    pub entry_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    /// Preferred action. It may forbid default execution and remain menu-only.
    pub action_id: String,
    pub actions: Vec<crate::Action>,
    pub aliases: Vec<String>,
    pub icon: Option<IconSource>,
}

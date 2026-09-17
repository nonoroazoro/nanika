use serde::{Deserialize, Serialize};

use crate::{CandidateKind, ContributionIcon, IconReference};

/// A bounded searchable entry contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub kind: CandidateKind,
    pub entry_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub action_id: String,
    pub aliases: Vec<String>,
    pub icon: Option<IconReference>,
    pub contribution_icon: Option<ContributionIcon>,
}

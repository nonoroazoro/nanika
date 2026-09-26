use serde::{Deserialize, Serialize};

/// Labels retain their width; descriptive text yields space to the candidate title.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "text", rename_all = "camelCase")]
pub enum CandidateSubtitle {
    Label(String),
    Description(String),
}

use serde::{Deserialize, Serialize};

/// Root Search entry semantics declared before activation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CandidateKind {
    Action,
    View,
}

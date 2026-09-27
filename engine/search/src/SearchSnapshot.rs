use crate::RankedCandidate;
use std::sync::Arc;

/// Immutable result snapshot tagged with the query generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSnapshot {
    pub generation: u64,
    pub result_revision: u64,
    pub instances: Arc<std::collections::HashMap<String, u64>>,
    pub normalized_query: String,
    pub pending_extensions: Vec<String>,
    /// Result identity is independent of progress-only publications.
    pub results: Arc<[RankedCandidate]>,
}

impl SearchSnapshot {
    pub fn authority(&self) -> crate::SearchAuthority {
        crate::SearchAuthority {
            generation: self.generation,
            result_revision: self.result_revision,
        }
    }
}

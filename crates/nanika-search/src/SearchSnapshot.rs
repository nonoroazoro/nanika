use crate::RankedCandidate;

/// Immutable result snapshot tagged with the query generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSnapshot {
    pub generation: u64,
    pub normalized_query: String,
    pub results: Vec<RankedCandidate>,
}

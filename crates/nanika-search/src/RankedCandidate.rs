use crate::Candidate;

/// A candidate after lexical and contextual ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedCandidate {
    pub candidate: Candidate,
    pub lexical_tier: u8,
    pub fuzzy_score: u32,
    pub contextual_boost: u32,
}

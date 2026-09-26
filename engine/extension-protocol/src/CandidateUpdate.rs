use crate::Candidate;

/// A query response either establishes its baseline or patches existing entry identities.
#[derive(Debug, Default)]
pub struct CandidateUpdate {
    pub replace: bool,
    pub removed: Vec<String>,
    pub entries: Vec<Candidate>,
}

use nanika_protocol::Candidate;

use crate::ApplicationEntry;

pub fn select_candidates(entries: &[ApplicationEntry], _query: &str) -> Vec<Candidate> {
    entries.iter().map(ApplicationEntry::candidate).collect()
}

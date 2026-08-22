use crate::{Candidate, normalize_history_key};

/// Stable identity for one action in one punctuation-preserving query context.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsageKey {
    pub extension_id: String,
    pub entry_id: String,
    pub action_id: String,
    pub query_context: String,
}

impl UsageKey {
    pub fn new(extension_id: &str, entry_id: &str, action_id: &str, query_context: &str) -> Self {
        Self {
            extension_id: extension_id.to_owned(),
            entry_id: entry_id.to_owned(),
            action_id: action_id.to_owned(),
            query_context: normalize_history_key(query_context),
        }
    }

    pub(crate) fn for_candidate(candidate: &Candidate, query_context: &str) -> Self {
        Self {
            extension_id: candidate.extension_id().to_owned(),
            entry_id: candidate.entry_id().to_owned(),
            action_id: candidate.action_id().to_owned(),
            query_context: normalize_history_key(query_context),
        }
    }
}

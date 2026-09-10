use std::collections::HashSet;

pub(crate) struct PendingSearchQuery {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) expected_extensions: HashSet<String>,
}

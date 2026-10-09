/// Semantic identity of one authoritative ranked result set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAuthority {
    pub generation: u64,
    pub result_revision: u64
}

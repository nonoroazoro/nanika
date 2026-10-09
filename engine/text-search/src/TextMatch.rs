/// Ordered lexical quality: exact, prefix, word-prefix/cross-field, then fuzzy.
/// Usage and destination preferences belong to the caller, not the matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextMatch {
    pub tier: u8,
    pub score: u32
}

/// Normalize identity while preserving punctuation significant to commands and scripts.
pub fn normalize_history_key(value: &str) -> String {
    value.trim().chars().flat_map(char::to_lowercase).collect()
}

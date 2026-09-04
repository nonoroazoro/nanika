/// Normalize text for lexical search matching.
pub fn normalize_query(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut pending_space = false;
    for character in value.chars() {
        if character.is_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            pending_space = false;
            normalized.extend(character.to_lowercase());
        } else {
            pending_space = true;
        }
    }
    normalized
}

/// Normalize identity while preserving punctuation significant to commands and scripts.
pub fn normalize_history_key(value: &str) -> String {
    value.trim().chars().flat_map(char::to_lowercase).collect()
}

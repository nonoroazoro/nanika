use std::borrow::Cow;

pub fn normalize_query(value: &str) -> String {
    let value = _without_isolates(value);
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

/// Bidi isolates affect presentation, not visible word boundaries. Only search
/// copies omit them; the original Fluent output and user content remain intact.
pub(crate) fn _without_isolates(value: &str) -> Cow<'_, str> {
    if value
        .chars()
        .any(|character| matches!(character, '\u{2066}'..='\u{2069}'))
    {
        // Removing isolates cannot increase byte length. Reserve once instead of
        // repeatedly growing a string from a filtered iterator with no lower bound.
        let mut stripped = String::with_capacity(value.len());
        stripped.extend(
            value
                .chars()
                .filter(|character| !matches!(character, '\u{2066}'..='\u{2069}'))
        );
        Cow::Owned(stripped)
    } else {
        Cow::Borrowed(value)
    }
}

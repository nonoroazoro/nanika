use crate::normalize_query;

/// Prepared query shared by launchers and Settings, without romanization or usage policy.
pub struct TextQuery {
    _normalized: String,
    pub(crate) _terms: Vec<String>,
}

impl TextQuery {
    pub fn new(raw: &str) -> Self {
        let normalized = normalize_query(raw);
        let terms = _cross_field_terms(&normalized)
            .into_iter()
            .map(str::to_owned)
            .collect();
        Self {
            _normalized: normalized,
            _terms: terms,
        }
    }

    pub fn normalized(&self) -> &str {
        &self._normalized
    }

    pub fn into_normalized(self) -> String {
        self._normalized
    }
}

// Split explicit terms and adjacent Han/non-Han text, without treating accented
// Latin letters as a separate script from ASCII letters.
fn _cross_field_terms(query: &str) -> Vec<&str> {
    if !query.contains(' ') {
        if query.is_ascii() {
            return Vec::new();
        }
        let mut scripts = query.chars().map(_is_han);
        let first = scripts.next();
        if scripts.all(|han| Some(han) == first) {
            return Vec::new();
        }
    }

    let mut terms = Vec::new();
    let mut start = 0;
    let mut previous_han = None;
    for (offset, character) in query.char_indices() {
        if character == ' ' {
            if start < offset {
                terms.push(&query[start..offset]);
            }
            start = offset + 1;
            previous_han = None;
            continue;
        }
        let han = _is_han(character);
        if previous_han.is_some_and(|previous| previous != han) {
            terms.push(&query[start..offset]);
            start = offset;
        }
        previous_han = Some(han);
    }
    if start < query.len() {
        terms.push(&query[start..]);
    }
    // Repeated terms impose no additional condition. Compare longer terms first
    // so an absent, selective term avoids scanning every short term per candidate.
    terms
        .sort_unstable_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    terms.dedup();
    if terms.len() < 2 { Vec::new() } else { terms }
}

fn _is_han(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
            | 0x30000..=0x323af
    )
}

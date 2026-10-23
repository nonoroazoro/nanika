use crate::{TextMatch, TextQuery, TextValue};
use nucleo_matcher::{Config, Matcher};

/// Reuses scratch memory for the app list's lexical and cross-field matching rules.
/// Values are normalized once by their catalog owner; no aliases are generated here.
pub struct TextMatcher {
    _matcher: Matcher
}

impl TextMatcher {
    pub fn new() -> Self {
        Self {
            _matcher: Matcher::new(Config::DEFAULT)
        }
    }

    pub fn score(&mut self, query: &TextQuery, values: &[TextValue]) -> Option<TextMatch> {
        if query.normalized().is_empty() {
            return Some(TextMatch { tier: 0, score: 0 });
        }
        let lexical = values
            .iter()
            .filter_map(|value| _lexical_match_value(query, value, &mut self._matcher))
            .max();
        let best = if lexical.is_some_and(|(tier, _)| tier >= 1) {
            lexical
        } else {
            lexical.max(_match_cross_field(&query._terms, values))
        };
        best.map(|(tier, score)| TextMatch { tier, score })
    }
}

impl Default for TextMatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn _match_cross_field(terms: &[String], values: &[TextValue]) -> Option<(u8, u32)> {
    (!terms.is_empty()
        && terms
            .iter()
            .all(|term| values.iter().any(|value| value.normalized().contains(term))))
    .then_some((1, u32::MAX - 3))
}

fn _lexical_match_value(
    query: &TextQuery,
    value: &TextValue,
    matcher: &mut Matcher
) -> Option<(u8, u32)> {
    let text = value.normalized();
    let normalized_query = query.normalized();
    if text == normalized_query {
        return Some((3, u32::MAX));
    }
    if text.starts_with(normalized_query) {
        return Some((2, u32::MAX - 1));
    }
    if text
        .split_whitespace()
        .any(|token| token.starts_with(normalized_query))
    {
        return Some((1, u32::MAX - 2));
    }
    let score = matcher
        .fuzzy_match(value.characters(), query._value.characters())
        .map(u32::from)?;
    (score >= query._fuzzy_cutoff).then_some((0, score))
}

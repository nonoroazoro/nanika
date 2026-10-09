use crate::{TextMatch, TextQuery};
use nucleo_matcher::{Config, Matcher, Utf32Str};

const MIN_FUZZY_SCORE_PER_CHARACTER: u32 = 12;

/// Reuses scratch memory for the app list's lexical and cross-field matching rules.
/// Values are normalized once by their catalog owner; no aliases are generated here.
pub struct TextMatcher {
    _matcher: Matcher,
    _haystack_buffer: Vec<char>,
    _query_buffer: Vec<char>
}

impl TextMatcher {
    pub fn new() -> Self {
        Self {
            _matcher: Matcher::new(Config::DEFAULT),
            _haystack_buffer: Vec::new(),
            _query_buffer: Vec::new()
        }
    }

    pub fn score(&mut self, query: &TextQuery, values: &[String]) -> Option<TextMatch> {
        if query.normalized().is_empty() {
            return Some(TextMatch { tier: 0, score: 0 });
        }
        let lexical = values
            .iter()
            .filter_map(|value| {
                _lexical_match_value(
                    query.normalized(),
                    value,
                    &mut self._matcher,
                    &mut self._haystack_buffer,
                    &mut self._query_buffer
                )
            })
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

fn _match_cross_field(terms: &[String], values: &[String]) -> Option<(u8, u32)> {
    (!terms.is_empty()
        && terms
            .iter()
            .all(|term| values.iter().any(|value| value.contains(term))))
    .then_some((1, u32::MAX - 3))
}

fn _lexical_match_value(
    query: &str,
    value: &str,
    matcher: &mut Matcher,
    haystack_buffer: &mut Vec<char>,
    query_buffer: &mut Vec<char>
) -> Option<(u8, u32)> {
    if value == query {
        return Some((3, u32::MAX));
    }
    if value.starts_with(query) {
        return Some((2, u32::MAX - 1));
    }
    if value
        .split_whitespace()
        .any(|token| token.starts_with(query))
    {
        return Some((1, u32::MAX - 2));
    }
    let score = matcher
        .fuzzy_match(
            Utf32Str::new(value, haystack_buffer),
            Utf32Str::new(query, query_buffer)
        )
        .map(u32::from)?;
    (score >= _fuzzy_cutoff(query)).then_some((0, score))
}

fn _fuzzy_cutoff(query: &str) -> u32 {
    u32::try_from(query.chars().count())
        .unwrap_or(u32::MAX)
        .saturating_mul(MIN_FUZZY_SCORE_PER_CHARACTER)
}

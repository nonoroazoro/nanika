use std::cmp::Ordering;

use nucleo_matcher::{Matcher, Utf32Str};

use crate::constants::{MIN_FUZZY_SCORE_PER_CHARACTER, RECENCY_HALF_LIFE_DAYS};
use crate::{
    Candidate, MatchContext, RankedCandidate, SearchSnapshot, UsageKey, UsageMap, UsageStat,
    normalize_query,
};

pub(crate) fn rank<'a>(
    generation: u64,
    query: &str,
    candidates: impl Iterator<Item = &'a Candidate>,
    usage: &UsageMap,
    now: u64,
    context: &mut MatchContext,
) -> SearchSnapshot {
    let normalized_query = normalize_query(query);
    let cross_field_terms = cross_field_terms(&normalized_query);
    let mut scored = candidates
        .filter_map(|candidate| {
            let lexical = lexical_match(
                &normalized_query,
                candidate,
                &mut context.matcher,
                &mut context.haystack_buffer,
                &mut context.query_buffer,
            );
            let (lexical_tier, fuzzy_score) = if lexical.is_some_and(|(tier, _)| tier >= 1) {
                lexical
            } else {
                lexical.max(match_cross_field(&cross_field_terms, candidate))
            }?;
            let contextual_boost = usage
                .get(&UsageKey::for_candidate(candidate, query))
                .map_or(0, |stat| contextual_boost(*stat, now));
            Some((candidate, lexical_tier, fuzzy_score, contextual_boost))
        })
        .collect::<Vec<_>>();

    scored.sort_by(compare_scored);

    SearchSnapshot {
        generation,
        normalized_query,
        results: scored
            .into_iter()
            .map(
                |(candidate, lexical_tier, fuzzy_score, contextual_boost)| RankedCandidate {
                    candidate: candidate.clone(),
                    lexical_tier,
                    fuzzy_score,
                    contextual_boost,
                },
            )
            .collect(),
    }
}

// Split explicit terms and adjacent Han/non-Han text, without treating accented
// Latin letters as a separate script from ASCII letters.
fn cross_field_terms(query: &str) -> Vec<&str> {
    if !query.contains(' ') {
        if query.is_ascii() {
            return Vec::new();
        }
        let mut scripts = query.chars().map(is_han);
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
        let han = is_han(character);
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

fn is_han(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
            | 0x30000..=0x323af
    )
}

fn match_cross_field(terms: &[&str], candidate: &Candidate) -> Option<(u8, u32)> {
    (!terms.is_empty()
        && terms.iter().all(|term| {
            candidate
                .search_values()
                .iter()
                .any(|value| value.contains(term))
        }))
    .then_some((1, u32::MAX - 3))
}

fn compare_scored(
    left: &(&Candidate, u8, u32, u32),
    right: &(&Candidate, u8, u32, u32),
) -> Ordering {
    right
        .1
        .cmp(&left.1)
        .then_with(|| right.3.cmp(&left.3))
        .then_with(|| right.2.cmp(&left.2))
        .then_with(|| left.0.title().cmp(right.0.title()))
        .then_with(|| left.0.extension_id().cmp(right.0.extension_id()))
        .then_with(|| left.0.entry_id().cmp(right.0.entry_id()))
        .then_with(|| left.0.action_id().cmp(right.0.action_id()))
}

fn lexical_match(
    query: &str,
    candidate: &Candidate,
    matcher: &mut Matcher,
    haystack_buffer: &mut Vec<char>,
    query_buffer: &mut Vec<char>,
) -> Option<(u8, u32)> {
    if query.is_empty() {
        return Some((0, 0));
    }
    candidate
        .search_values()
        .iter()
        .filter_map(|value| {
            lexical_match_value(query, value, matcher, haystack_buffer, query_buffer)
        })
        .max()
}

fn lexical_match_value(
    query: &str,
    value: &str,
    matcher: &mut Matcher,
    haystack_buffer: &mut Vec<char>,
    query_buffer: &mut Vec<char>,
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
            Utf32Str::new(query, query_buffer),
        )
        .map(u32::from)?;
    (score >= fuzzy_cutoff(query)).then_some((0, score))
}

fn fuzzy_cutoff(query: &str) -> u32 {
    u32::try_from(query.chars().count())
        .unwrap_or(u32::MAX)
        .saturating_mul(MIN_FUZZY_SCORE_PER_CHARACTER)
}

fn contextual_boost(stat: UsageStat, now: u64) -> u32 {
    let count = stat.execution_count;
    let days = now.saturating_sub(stat.last_executed_at) / 86_400;
    let half_lives = u32::try_from(days / RECENCY_HALF_LIFE_DAYS).unwrap_or(u32::MAX);
    let recency = 1_000_u32.checked_shr(half_lives).unwrap_or(0);
    count.saturating_mul(10).saturating_add(recency)
}

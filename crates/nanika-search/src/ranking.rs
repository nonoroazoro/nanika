use std::cmp::Ordering;

use nucleo_matcher::{Matcher, Utf32Str};

use crate::constants::{
    MAX_RESULTS, MAX_USAGE_COUNT, MIN_FUZZY_SCORE_PER_CHARACTER, RECENCY_HALF_LIFE_DAYS,
};
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
    let mut scored = candidates
        .filter_map(|candidate| {
            let (lexical_tier, fuzzy_score) = lexical_match(
                &normalized_query,
                candidate,
                &mut context.matcher,
                &mut context.haystack_buffer,
                &mut context.query_buffer,
            )?;
            let contextual_boost = usage
                .get(&UsageKey::for_candidate(candidate, query))
                .map_or(0, |stat| contextual_boost(*stat, now));
            Some((candidate, lexical_tier, fuzzy_score, contextual_boost))
        })
        .collect::<Vec<_>>();

    if scored.len() > MAX_RESULTS {
        scored.select_nth_unstable_by(MAX_RESULTS, compare_scored);
        scored.truncate(MAX_RESULTS);
    }
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
    let count = stat.execution_count.min(MAX_USAGE_COUNT);
    let days = now.saturating_sub(stat.last_executed_at) / 86_400;
    let half_lives = u32::try_from(days / RECENCY_HALF_LIFE_DAYS).unwrap_or(u32::MAX);
    let recency = 1_000_u32.checked_shr(half_lives).unwrap_or(0);
    count.saturating_mul(10).saturating_add(recency)
}

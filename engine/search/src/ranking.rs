use std::cmp::Ordering;

use nanika_text_search::{TextMatch, TextMatcher, TextQuery};

use crate::constants::RECENCY_HALF_LIFE_DAYS;
use crate::{
    Candidate, RankedCandidate, SearchSnapshot, UsageKey, UsageMap, UsageStat,
    normalize_history_key
};

pub(crate) fn rank<'a>(
    generation: u64,
    query: &str,
    candidates: impl Iterator<Item = &'a Candidate>,
    usage: &UsageMap,
    now: u64,
    context: &mut TextMatcher,
    cancelled: impl Fn() -> bool
) -> Option<SearchSnapshot> {
    let text_query = TextQuery::new(query);
    let mixed_query = nanika_text_search::romanized_query(query);
    let mut usage_key = UsageKey::new("", "", "", "");
    usage_key.query_context = normalize_history_key(query);
    let mut scored = Vec::new();
    for (index, candidate) in candidates.enumerate() {
        if index % 256 == 0 && cancelled() {
            return None;
        }
        let matched = (|| {
            let mut lexical = context.score(&text_query, candidate.search_values());
            if let Some(matched) =
                nanika_text_search::find_romanized_match(candidate.readings(), &mixed_query)
            {
                let (tier, score) = match matched {
                    nanika_text_search::RomanizedMatch::Exact => (3, u32::MAX),
                    nanika_text_search::RomanizedMatch::Prefix(_) => (2, u32::MAX - 1),
                    nanika_text_search::RomanizedMatch::Infix(_) => (1, u32::MAX - 2)
                };
                lexical = lexical.max(Some(TextMatch { tier, score }));
            }
            let TextMatch {
                tier: lexical_tier,
                score: fuzzy_score
            } = lexical?;
            let contextual_boost = if usage.is_empty() {
                0
            } else {
                for (target, value) in [
                    (&mut usage_key.extension_id, candidate.extension_id()),
                    (&mut usage_key.entry_id, candidate.entry_id()),
                    (&mut usage_key.action_id, candidate.action_id())
                ] {
                    target.clear();
                    target.push_str(value);
                }
                usage
                    .get(&usage_key)
                    .map_or(0, |stat| contextual_boost(*stat, now))
            };
            Some((candidate, lexical_tier, fuzzy_score, contextual_boost))
        })();
        if let Some(matched) = matched {
            scored.push(matched);
        }
    }
    if cancelled() {
        return None;
    }
    scored.sort_unstable_by(compare_scored);
    if cancelled() {
        return None;
    }

    Some(SearchSnapshot {
        result_revision: 0,
        instances: Default::default(),
        generation,
        normalized_query: text_query.into_normalized(),
        pending_extensions: Vec::new(),
        results: scored
            .into_iter()
            .map(
                |(candidate, lexical_tier, fuzzy_score, contextual_boost)| RankedCandidate {
                    candidate: candidate.clone(),
                    lexical_tier,
                    fuzzy_score,
                    contextual_boost
                }
            )
            .collect()
    })
}

fn compare_scored(
    left: &(&Candidate, u8, u32, u32),
    right: &(&Candidate, u8, u32, u32)
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

fn contextual_boost(stat: UsageStat, now: u64) -> u32 {
    let count = stat.execution_count;
    let days = now.saturating_sub(stat.last_executed_at) / 86_400;
    let half_lives = u32::try_from(days / RECENCY_HALF_LIFE_DAYS).unwrap_or(u32::MAX);
    let recency = 1_000_u32.checked_shr(half_lives).unwrap_or(0);
    count.saturating_mul(10).saturating_add(recency)
}

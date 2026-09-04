use std::cmp::Ordering;
use std::collections::HashSet;

use nanika_protocol::Candidate;

use crate::ApplicationEntry;
use crate::normalization::normalize_name;

pub fn select_candidates(
    entries: &[ApplicationEntry],
    query: &str,
    limit: usize,
) -> Vec<Candidate> {
    if entries.len() <= limit || query.is_empty() {
        return entries
            .iter()
            .take(limit)
            .map(ApplicationEntry::candidate)
            .collect();
    }
    let query = normalize_name(query);
    if query.is_empty() {
        return entries
            .iter()
            .take(limit)
            .map(ApplicationEntry::candidate)
            .collect();
    }
    let mut matched = entries
        .iter()
        .filter_map(|entry| entry_match_rank(&query, entry).map(|rank| (entry, rank)))
        .collect::<Vec<_>>();
    if matched.len() > limit {
        matched.select_nth_unstable_by(limit, compare_matches);
        matched.truncate(limit);
    }
    matched.sort_unstable_by(compare_matches);

    let mut selected = matched
        .iter()
        .map(|(entry, _)| entry.candidate())
        .collect::<Vec<_>>();
    if selected.len() == limit {
        return selected;
    }
    let matched_ids = matched
        .into_iter()
        .map(|(entry, _)| entry.entry_id.as_str())
        .collect::<HashSet<_>>();
    selected.extend(
        entries
            .iter()
            .filter(|entry| !matched_ids.contains(entry.entry_id.as_str()))
            .take(limit - selected.len())
            .map(ApplicationEntry::candidate),
    );
    selected
}

fn entry_match_rank(query: &str, entry: &ApplicationEntry) -> Option<(u8, usize)> {
    std::iter::once(entry.normalized_name.as_str())
        .chain(entry.normalized_tokens.lines())
        .filter_map(|value| match_rank(query, value))
        .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
}

fn match_rank(query: &str, value: &str) -> Option<(u8, usize)> {
    if value == query {
        return Some((3, 0));
    }
    if value.starts_with(query) {
        return Some((2, value.len().saturating_sub(query.len())));
    }
    if value
        .split_whitespace()
        .any(|token| token.starts_with(query))
    {
        return Some((1, value.len().saturating_sub(query.len())));
    }
    is_subsequence(query, value).then_some((0, value.len().saturating_sub(query.len())))
}

fn is_subsequence(query: &str, value: &str) -> bool {
    let mut query = query.chars();
    let mut expected = query.next();
    for character in value.chars() {
        if expected == Some(character) {
            expected = query.next();
            if expected.is_none() {
                return true;
            }
        }
    }
    false
}

fn compare_matches(
    (left, left_rank): &(&ApplicationEntry, (u8, usize)),
    (right, right_rank): &(&ApplicationEntry, (u8, usize)),
) -> Ordering {
    right_rank
        .0
        .cmp(&left_rank.0)
        .then_with(|| left_rank.1.cmp(&right_rank.1))
        .then_with(|| left.normalized_name.cmp(&right.normalized_name))
        .then_with(|| left.entry_id.cmp(&right.entry_id))
}

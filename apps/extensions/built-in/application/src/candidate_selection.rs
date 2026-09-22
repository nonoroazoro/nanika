use nanika_protocol::Candidate;
use nanika_text_search::{RomanizedMatch, find_romanized_match, romanized_query};

use crate::ApplicationEntry;

pub fn select_candidates(entries: &[ApplicationEntry], query: &str) -> Vec<Candidate> {
    let mixed = romanized_query(query);
    entries
        .iter()
        .map(|entry| {
            let mut candidate = entry.candidate();
            if mixed.is_empty() {
                return candidate;
            }
            if let Some(matched) = find_romanized_match(&entry.search_readings, &mixed) {
                // Preserve the spelling position when handing the match to host ranking.
                let alias = match matched {
                    RomanizedMatch::Exact => query.to_owned(),
                    RomanizedMatch::Prefix(remainder) => format!("{query}{remainder}"),
                    RomanizedMatch::Infix(prefix) => format!("{prefix} {query}"),
                };
                if !candidate.aliases.contains(&alias) {
                    candidate.aliases.push(alias);
                }
            }
            candidate
        })
        .collect()
}

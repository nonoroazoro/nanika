use nanika_search::{Candidate, RankedCandidate, SearchSnapshot};

use crate::prepare_visible_results;

#[test]
fn preparation_supports_the_complete_root_result_limit_and_tracks_selection() {
    let snapshot = snapshot(7, 10);
    let prepared = prepare_visible_results(&snapshot, 7, 3).collect::<Vec<_>>();

    assert_eq!(prepared.len(), 10);
    assert!(prepared[3].2);
    assert_eq!(prepared[9].1.candidate.entry_id(), "entry-9");
}

#[test]
fn preparation_rejects_stale_generation() {
    let snapshot = snapshot(6, 10);

    assert_eq!(prepare_visible_results(&snapshot, 7, 0).count(), 0);
}

fn snapshot(generation: u64, count: usize) -> SearchSnapshot {
    SearchSnapshot {
        generation,
        normalized_query: "application".to_owned(),
        results: (0..count)
            .map(|index| RankedCandidate {
                candidate: Candidate::new(
                    "benchmark",
                    format!("entry-{index}"),
                    format!("Application {index}"),
                    "launch",
                    Vec::new(),
                ),
                lexical_tier: 1,
                fuzzy_score: 100,
                contextual_boost: 0,
            })
            .collect(),
    }
}

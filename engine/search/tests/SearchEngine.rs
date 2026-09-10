use nanika_search::{Candidate, SearchEngine, UsageKey, UsageMap, UsageStat, normalize_query};

fn candidate(entry_id: &str, title: &str, action_id: &str) -> Candidate {
    Candidate::new("test.extension", entry_id, title, action_id, Vec::new())
}

#[test]
fn normalization_collapses_punctuation_case_and_whitespace() {
    assert_eq!(normalize_query("  Git-Hub   DESKTOP "), "git hub desktop");
}

#[test]
fn aliases_receive_the_same_lexical_tiers_as_titles() {
    let entry = Candidate::new(
        "test.extension",
        "alias",
        "Calculator",
        "open",
        vec!["计算器".to_owned()],
    );
    let snapshot = SearchEngine::new().query("计算器", &[entry], &UsageMap::new(), 0);
    assert_eq!(snapshot.results[0].lexical_tier, 3);
}

#[test]
fn weak_fuzzy_matches_are_rejected() {
    let title = format!("a{}b{}c", "x".repeat(200), "y".repeat(200));
    let snapshot = SearchEngine::new().query(
        "abc",
        &[candidate("weak", &title, "open")],
        &UsageMap::new(),
        0,
    );
    assert!(snapshot.results.is_empty());
}

#[test]
fn lexical_tier_beats_contextual_frequency() {
    let candidates = vec![
        candidate("exact", "Cal", "open"),
        candidate("prefix", "Calendar", "open"),
    ];
    let mut usage = UsageMap::new();
    usage.insert(
        UsageKey::new("test.extension", "prefix", "open", "cal"),
        UsageStat {
            execution_count: 100,
            last_executed_at: 1_000,
        },
    );
    let snapshot = SearchEngine::new().query("cal", &candidates, &usage, 1_000);
    assert_eq!(snapshot.results[0].candidate.entry_id(), "exact");
}

#[test]
fn contextual_usage_reorders_candidates_inside_a_tier() {
    let candidates = vec![
        candidate("unused", "Tool", "unused"),
        candidate("used", "Tool", "used"),
    ];
    let mut usage = UsageMap::new();
    usage.insert(
        UsageKey::new("test.extension", "used", "used", "tool"),
        UsageStat {
            execution_count: 5,
            last_executed_at: 1_000,
        },
    );
    let snapshot = SearchEngine::new().query("tool", &candidates, &usage, 1_000);
    assert_eq!(snapshot.results[0].candidate.entry_id(), "used");
}

#[test]
fn prefix_and_token_matches_have_distinct_tiers() {
    let candidates = vec![
        candidate("token", "Open Calculator", "open"),
        candidate("prefix", "Calculator", "open"),
    ];
    let snapshot = SearchEngine::new().query("cal", &candidates, &UsageMap::new(), 0);
    assert_eq!(snapshot.results[0].candidate.entry_id(), "prefix");
    assert_eq!(snapshot.results[0].lexical_tier, 2);
    assert_eq!(snapshot.results[1].lexical_tier, 1);
}

#[test]
fn ranking_tie_breaker_is_stable() {
    let candidates = vec![
        candidate("b", "Tool", "open"),
        candidate("a", "Tool", "open"),
    ];
    let snapshot = SearchEngine::new().query("tool", &candidates, &UsageMap::new(), 0);
    assert_eq!(snapshot.results[0].candidate.entry_id(), "a");
}

#[test]
fn complete_catalog_remains_browsable_and_searchable() {
    let entries = (0..2_000)
        .map(|index| {
            candidate(
                &format!("entry-{index}"),
                &format!("Application {index:04}"),
                "open",
            )
        })
        .collect::<Vec<_>>();
    let mut engine = SearchEngine::new();
    for query in ["", "application"] {
        let snapshot = engine.query(query, &entries, &UsageMap::new(), 0);
        assert_eq!(snapshot.results.len(), entries.len());
        assert_eq!(
            snapshot.results.last().unwrap().candidate.entry_id(),
            "entry-1999"
        );
    }
    let snapshot = engine.query("Application 1999", &entries, &UsageMap::new(), 0);
    assert_eq!(snapshot.results[0].candidate.entry_id(), "entry-1999");
}

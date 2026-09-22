use nanika_search::{
    Candidate, CandidateKind, SearchEngine, UsageKey, UsageMap, UsageStat, normalize_query,
};

fn candidate(entry_id: &str, title: &str, action_id: &str) -> Candidate {
    Candidate::new(
        CandidateKind::Action,
        "test.extension",
        entry_id,
        title,
        action_id,
        Vec::new(),
    )
}

#[test]
fn normalization_collapses_punctuation_case_and_whitespace() {
    assert_eq!(normalize_query("  Git-Hub   DESKTOP "), "git hub desktop");
}

#[test]
fn aliases_receive_the_same_lexical_tiers_as_titles() {
    let entry = Candidate::new(
        CandidateKind::Action,
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
fn different_names_of_one_candidate_satisfy_all_query_terms() {
    let candidates = [
        Candidate::new(
            CandidateKind::Action,
            "test.extension",
            "music",
            "音乐",
            "open",
            vec!["music".to_owned(), "yinyue".to_owned()],
        ),
        Candidate::new(
            CandidateKind::Action,
            "test.extension",
            "unrelated",
            "音乐盒",
            "open",
            vec!["podcast".to_owned()],
        ),
    ];
    let mut engine = SearchEngine::new();
    for query in ["音乐 music", "音乐music", "yin music", "music 音乐"] {
        let snapshot = engine.query(query, &candidates, &UsageMap::new(), 0);
        assert_eq!(snapshot.results.len(), 1, "query: {query}");
        assert_eq!(snapshot.results[0].candidate.entry_id(), "music");
    }
    assert!(
        engine
            .query("音乐 missing", &candidates, &UsageMap::new(), 0)
            .results
            .is_empty()
    );
}

#[test]
fn short_terms_follow_the_same_cross_name_rule() {
    let candidates = [Candidate::new(
        CandidateKind::Action,
        "test.extension",
        "letter",
        "A",
        "open",
        vec!["Music".to_owned()],
    )];
    for query in ["a music", "music a"] {
        let snapshot = SearchEngine::new().query(query, &candidates, &UsageMap::new(), 0);
        assert_eq!(snapshot.results[0].candidate.entry_id(), "letter");
    }
}

#[test]
fn accented_latin_word_is_not_split_across_unrelated_names() {
    let candidates = [Candidate::new(
        CandidateKind::Action,
        "test.extension",
        "unrelated",
        "R",
        "open",
        vec!["ésumé".to_owned()],
    )];
    let snapshot = SearchEngine::new().query("résumé", &candidates, &UsageMap::new(), 0);
    assert!(snapshot.results.is_empty());
}

#[test]
fn a_contiguous_name_still_ranks_above_cross_name_terms() {
    let candidates = [
        candidate("contiguous", "音乐 Music", "open"),
        Candidate::new(
            CandidateKind::Action,
            "test.extension",
            "separate",
            "音乐",
            "open",
            vec!["music".to_owned()],
        ),
    ];
    let snapshot = SearchEngine::new().query("音乐 music", &candidates, &UsageMap::new(), 0);
    assert_eq!(snapshot.results[0].candidate.entry_id(), "contiguous");
    assert_eq!(snapshot.results[1].candidate.entry_id(), "separate");
}

#[test]
fn cross_name_terms_beat_a_weak_single_name_fuzzy_match() {
    let candidates = [Candidate::new(
        CandidateKind::Action,
        "test.extension",
        "music",
        "音乐",
        "open",
        vec!["music".to_owned(), "音乐 other music".to_owned()],
    )];
    let snapshot = SearchEngine::new().query("音乐 music", &candidates, &UsageMap::new(), 0);
    assert_eq!(snapshot.results[0].lexical_tier, 1);
}

#[test]
fn repeated_query_terms_do_not_create_cross_name_matches() {
    let candidates = [Candidate::new(
        CandidateKind::Action,
        "test.extension",
        "unrelated",
        "Alpha",
        "open",
        vec!["Beta".to_owned()],
    )];
    let query = "a ".repeat(2_048);
    let snapshot = SearchEngine::new().query(&query, &candidates, &UsageMap::new(), 0);
    assert!(snapshot.results.is_empty());
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

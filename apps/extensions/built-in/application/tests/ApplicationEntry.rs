use crate::ApplicationEntry;
use crate::ApplicationEntryData;
use nanika_search::{Candidate as SearchCandidate, CandidateKind, SearchEngine, UsageMap};

#[test]
fn location_actions_follow_the_application_kind_without_launching() {
    let mut app = entry(0, "Example");
    for kind in ["executable", "windows-shell-link", "macos-bundle"] {
        app.launch_kind = kind.to_owned();
        assert!(
            app.actions()
                .iter()
                .any(|action| action.id == "application.reveal")
        );
        assert_eq!(
            app.actions()
                .iter()
                .any(|action| action.id == "application.revealTarget"),
            kind == "windows-shell-link"
        );
        assert!(matches!(app.host_request("application.reveal").unwrap(),
            nanika_protocol::HostServiceRequest::RevealPath { path } if path == app.target_path));
    }
    app.launch_kind = "windows-packaged".to_owned();
    assert_eq!(app.actions().len(), 1);
    assert!(app.host_request("application.reveal").is_err());
    assert!(app.host_request("application.revealTarget").is_err());
    assert!(app.host_request("unknown").is_err());
}

#[test]
fn complete_catalog_keeps_exact_matches_available_to_host_ranking() {
    let mut entries = (0..5_001)
        .map(|index| entry(index, &format!("Application {index:04}")))
        .collect::<Vec<_>>();
    entries.push(entry(5_001, "Zettelkasten"));

    let selected = entries
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>();

    assert_eq!(selected.len(), 5_002);
    let candidate = selected
        .last()
        .expect("exact match should remain available");
    assert_eq!(candidate.title, "Zettelkasten");
    assert_eq!(
        candidate.icon,
        Some(nanika_protocol::IconSource::Empty),
        "discovery reserves the icon slot without inspecting unprepared icons"
    );
}

#[test]
fn small_snapshots_preserve_the_full_host_ranking_input() {
    let entries = [entry(0, "Zulu"), entry(1, "Alpha")];

    let selected = entries
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>();

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].title, "Zulu");
    assert_eq!(selected[1].title, "Alpha");
}

#[test]
fn localized_names_keep_complete_original_names_searchable() {
    let mut localized = entry(0, "图书");
    localized.normalized_tokens = "books\nbook reader".to_owned();

    let candidate = localized.candidate();

    assert_eq!(candidate.title, "图书");
    assert_eq!(candidate.aliases, ["books", "book reader"]);
}

#[test]
fn complete_catalog_keeps_aliases_available_to_host_ranking() {
    let mut entries = (0..5_001)
        .map(|index| entry(index, &format!("Application {index:04}")))
        .collect::<Vec<_>>();
    let mut localized = entry(5_001, "图书");
    localized.normalized_tokens = "books".to_owned();
    entries.push(localized);

    let selected = entries
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>();

    let candidate = selected.last().expect("alias should remain available");
    assert_eq!(candidate.title, "图书");
    assert_eq!(candidate.aliases, ["books"]);
}

#[test]
fn romanized_aliases_work_with_the_existing_host_ranker() {
    let entries = [entry(0, "同步"), entry(1, "音乐"), entry(2, "朝阳")];
    for (query, title) in [
        ("tongbu", "同步"),
        ("tonbu", "同步"),
        ("tb", "同步"),
        ("yinyue", "音乐"),
        ("yy", "音乐"),
        ("zhaoyang", "朝阳"),
        ("chaoyang", "朝阳"),
        ("同bu", "同步"),
        ("tong步", "同步"),
        ("t步", "同步"),
        ("音yue", "音乐"),
    ] {
        let candidates = entries
            .iter()
            .map(ApplicationEntry::candidate)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|candidate| {
                SearchCandidate::new(
                    CandidateKind::Action,
                    "application",
                    candidate.entry_id,
                    candidate.title,
                    candidate.action_id,
                    candidate.actions,
                    candidate.aliases,
                )
            })
            .collect::<Vec<_>>();
        let ranked = SearchEngine::new().query(query, &candidates, &UsageMap::new(), 0);
        assert_eq!(
            ranked
                .results
                .first()
                .map(|result| result.candidate.title()),
            Some(title),
            "query: {query}"
        );
    }
}

#[test]
fn alternate_chinese_names_gain_their_own_romanization() {
    let mut entry = entry(0, "Music");
    entry.normalized_tokens = "音乐".to_owned();
    let candidate = entry.candidate();
    let candidates = [SearchCandidate::new(
        CandidateKind::Action,
        "application",
        candidate.entry_id,
        candidate.title,
        candidate.action_id,
        candidate.actions,
        candidate.aliases,
    )];
    for query in ["音乐", "yinyue", "yy"] {
        assert_eq!(
            SearchEngine::new()
                .query(query, &candidates, &UsageMap::new(), 0)
                .results
                .len(),
            1
        );
    }
}

#[test]
fn localized_and_romanized_names_can_be_combined_in_one_query() {
    let mut entry = entry(0, "音乐");
    entry.normalized_tokens = "music".to_owned();
    let candidates = [entry]
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|candidate| {
            SearchCandidate::new(
                CandidateKind::Action,
                "application",
                candidate.entry_id,
                candidate.title,
                candidate.action_id,
                candidate.actions,
                candidate.aliases,
            )
        })
        .collect::<Vec<_>>();
    for query in ["音乐 music", "音乐music", "yin music"] {
        let ranked = SearchEngine::new().query(query, &candidates, &UsageMap::new(), 0);
        assert_eq!(
            ranked.results[0].candidate.title(),
            "音乐",
            "query: {query}"
        );
    }
}

#[test]
fn mixed_chinese_and_latin_names_match_without_storing_combinations() {
    let entry = entry(0, "同步 Sync");
    for query in ["同步sync", "同bu sync"] {
        let candidate = [entry.clone()]
            .iter()
            .map(ApplicationEntry::candidate)
            .collect::<Vec<_>>()
            .pop()
            .expect("complete catalog");
        let candidates = [SearchCandidate::new(
            CandidateKind::Action,
            "application",
            candidate.entry_id,
            candidate.title,
            candidate.action_id,
            candidate.actions,
            candidate.aliases,
        )];
        assert_eq!(
            SearchEngine::new()
                .query(query, &candidates, &UsageMap::new(), 0)
                .results[0]
                .lexical_tier,
            3
        );
    }
}

#[test]
fn literal_mixed_name_keeps_its_exact_match() {
    let entries = [entry(0, "音yue"), entry(1, "音乐")];
    let candidates = entries
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|candidate| {
            SearchCandidate::new(
                CandidateKind::Action,
                "application",
                candidate.entry_id,
                candidate.title,
                candidate.action_id,
                candidate.actions,
                candidate.aliases,
            )
        })
        .collect::<Vec<_>>();
    let ranked = SearchEngine::new().query("音yue", &candidates, &UsageMap::new(), 0);
    assert_eq!(ranked.results[0].candidate.title(), "音yue");
}

#[test]
fn literal_mixed_name_ranks_above_cross_name_matches() {
    let mut entries = [entry(0, "同步"), entry(1, "t步"), entry(2, "步行")];
    entries[2].normalized_tokens = "同伴".to_owned();
    let initial_candidates = entries
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|candidate| {
            SearchCandidate::new(
                CandidateKind::Action,
                "application",
                candidate.entry_id,
                candidate.title,
                candidate.action_id,
                candidate.actions,
                candidate.aliases,
            )
        })
        .collect::<Vec<_>>();
    let initial_results =
        SearchEngine::new().query("t步", &initial_candidates, &UsageMap::new(), 0);
    assert_eq!(initial_results.results[0].candidate.title(), "t步");
    assert!(
        initial_results
            .results
            .iter()
            .any(|result| result.candidate.title() == "同步")
    );
}

#[test]
fn mixed_query_preserves_exact_prefix_and_infix_order() {
    let entries = [entry(0, "同步"), entry(1, "同步工具"), entry(2, "不同步")];
    let candidates = entries
        .iter()
        .map(ApplicationEntry::candidate)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|candidate| {
            SearchCandidate::new(
                CandidateKind::Action,
                "application",
                candidate.entry_id,
                candidate.title,
                candidate.action_id,
                candidate.actions,
                candidate.aliases,
            )
        })
        .collect::<Vec<_>>();
    let ranked = SearchEngine::new().query("同bu", &candidates, &UsageMap::new(), 0);
    assert_eq!(
        ranked
            .results
            .iter()
            .map(|result| (result.candidate.title(), result.lexical_tier))
            .collect::<Vec<_>>(),
        [("同步", 3), ("同步工具", 2), ("不同步", 1)]
    );
}

fn entry(index: usize, name: &str) -> ApplicationEntry {
    let normalized_name = name.to_lowercase();
    ApplicationEntry::new(ApplicationEntryData {
        entry_id: format!("app.{index}"),
        source_key: format!("source.{index}"),
        display_name: name.to_owned(),
        normalized_name: normalized_name.clone(),
        normalized_tokens: normalized_name,
        launch_kind: "executable".to_owned(),
        target_path: format!("Application-{index}.exe"),
        arguments_json: r#"{"kind":"structured","values":[]}"#.to_owned(),
        icon_key: "fallback".to_owned(),
        icon_source: None,
        icon_index: 0,
        priority: 0,
    })
}

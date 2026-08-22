use crate::{ApplicationEntry, select_candidates};

#[test]
fn exact_matches_beyond_the_snapshot_limit_remain_searchable() {
    let mut entries = (0..5_001)
        .map(|index| entry(index, &format!("Application {index:04}")))
        .collect::<Vec<_>>();
    entries.push(entry(5_001, "Zettelkasten"));

    let selected = select_candidates(&entries, "zettelkasten", 5_000);

    assert_eq!(selected.len(), 5_000);
    assert_eq!(selected[0].title, "Zettelkasten");
}

#[test]
fn small_snapshots_preserve_the_full_host_ranking_input() {
    let entries = vec![entry(0, "Zulu"), entry(1, "Alpha")];

    let selected = select_candidates(&entries, "alpha", 5_000);

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].title, "Zulu");
    assert_eq!(selected[1].title, "Alpha");
}

fn entry(index: usize, name: &str) -> ApplicationEntry {
    let normalized_name = name.to_lowercase();
    ApplicationEntry {
        entry_id: format!("app.{index}"),
        source_key: format!("source.{index}"),
        display_name: name.to_owned(),
        normalized_name: normalized_name.clone(),
        normalized_tokens: normalized_name,
        launch_kind: "executable".to_owned(),
        target_path: format!("Application-{index}.exe"),
        working_directory: None,
        arguments_json: r#"{"kind":"structured","values":[]}"#.to_owned(),
        bundle_id: None,
        icon_key: "fallback".to_owned(),
        file_identity: format!("file.{index}"),
        last_seen_at: 1,
        stale: false,
        icon_source: None,
        icon_index: 0,
        priority: 0,
    }
}

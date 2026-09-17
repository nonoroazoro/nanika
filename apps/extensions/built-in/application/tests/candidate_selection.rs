use crate::{ApplicationEntry, select_candidates};

#[test]
fn complete_catalog_keeps_exact_matches_available_to_host_ranking() {
    let mut entries = (0..5_001)
        .map(|index| entry(index, &format!("Application {index:04}")))
        .collect::<Vec<_>>();
    entries.push(entry(5_001, "Zettelkasten"));

    let selected = select_candidates(&entries, "zettelkasten");

    assert_eq!(selected.len(), 5_002);
    let candidate = selected
        .last()
        .expect("exact match should remain available");
    assert_eq!(candidate.title, "Zettelkasten");
    assert_eq!(
        candidate.icon.as_ref().map(|icon| icon.key()),
        Some("fallback")
    );
}

#[test]
fn small_snapshots_preserve_the_full_host_ranking_input() {
    let entries = vec![entry(0, "Zulu"), entry(1, "Alpha")];

    let selected = select_candidates(&entries, "alpha");

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

    let selected = select_candidates(&entries, "books");

    let candidate = selected.last().expect("alias should remain available");
    assert_eq!(candidate.title, "图书");
    assert_eq!(candidate.aliases, ["books"]);
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
        icon_source: None,
        icon_index: 0,
        priority: 0,
    }
}

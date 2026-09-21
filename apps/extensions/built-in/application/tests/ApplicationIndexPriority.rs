use super::*;

fn entry(id: &str) -> ApplicationEntry {
    ApplicationEntry {
        entry_id: id.to_owned(),
        source_key: id.to_owned(),
        display_name: id.to_owned(),
        normalized_name: id.to_owned(),
        normalized_tokens: id.to_owned(),
        launch_kind: "macos-bundle".to_owned(),
        target_path: format!("/{id}.app"),
        working_directory: None,
        arguments_json: "{\"kind\":\"structured\",\"values\":[]}".to_owned(),
        bundle_id: None,
        icon_key: id.to_owned(),
        icon_source: None,
        icon_index: 0,
        priority: 0,
    }
}

#[test]
fn host_visible_entries_move_to_the_front_in_host_order() {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-priority-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(root.join("index.db")).unwrap(),
        IconCache::new(root.join("icons")),
    );
    index.pending_icons = vec![entry("a"), entry("b"), entry("c"), entry("d")];
    assert!(
        index
            .pending_icons
            .iter()
            .any(|entry| entry.entry_id == "b")
    );
    assert!(
        !index
            .pending_icons
            .iter()
            .any(|entry| entry.entry_id == "missing")
    );
    index.prioritize_pending_icons(&["d".to_owned(), "b".to_owned()]);
    assert_eq!(
        index
            .pending_icons
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>(),
        ["d", "b", "a", "c"]
    );
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}

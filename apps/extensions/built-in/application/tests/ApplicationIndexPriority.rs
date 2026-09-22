use super::*;
use crate::ScanReport;

fn entry(id: &str) -> ApplicationEntry {
    ApplicationEntry {
        entry_id: id.to_owned(),
        source_key: id.to_owned(),
        display_name: id.to_owned(),
        normalized_name: id.to_owned(),
        normalized_tokens: id.to_owned(),
        search_readings: Vec::new(),
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
    index.prepared_entries = Some(vec![entry("a"), entry("b"), entry("c"), entry("d")]);
    index.pending_icons = vec![0, 1, 2, 3];
    assert!(
        index
            .pending_icons
            .iter()
            .any(|&index_id| index.prepared_entries.as_ref().unwrap()[index_id].entry_id == "b")
    );
    assert!(
        !index
            .pending_icons
            .iter()
            .any(
                |&index_id| index.prepared_entries.as_ref().unwrap()[index_id].entry_id
                    == "missing"
            )
    );
    index.prioritize_pending_icons(&["d".to_owned(), "b".to_owned()]);
    assert_eq!(
        index
            .pending_icons
            .iter()
            .map(
                |&index_id| index.prepared_entries.as_ref().unwrap()[index_id]
                    .entry_id
                    .as_str()
            )
            .collect::<Vec<_>>(),
        ["d", "b", "a", "c"]
    );
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_names_reuse_readings_across_scans_and_icon_batches() {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-readings-cache-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(root.join("index.db")).unwrap(),
        IconCache::new(root.join("icons")),
    );
    let mut application = entry("sync");
    application.display_name = "同步".to_owned();
    application.normalized_name = "同步".to_owned();
    application.normalized_tokens = "同步".to_owned();

    commit(&mut index, 1, &application);
    let initial = index.load_presentable().unwrap();
    assert_eq!(initial[0].search_readings[0].full, "tongbu");
    let allocation = index.prepared_entries.as_ref().unwrap()[0].search_readings[0]
        .full
        .as_ptr();

    commit(&mut index, 2, &application);
    index
        .cache_scanned_entries(vec![application.clone()])
        .unwrap();
    assert_eq!(
        index.prepared_entries.as_ref().unwrap()[0].search_readings[0]
            .full
            .as_ptr(),
        allocation,
        "unchanged search inputs must move cached readings, not rebuild them"
    );
    index.populate_icon_batch(&AtomicU64::new(0), 2, 1).unwrap();
    assert_eq!(
        index.prepared_entries.as_ref().unwrap()[0].search_readings[0]
            .full
            .as_ptr(),
        allocation,
        "icon-only work must not rebuild search readings"
    );
    assert_eq!(
        index.load_presentable().unwrap()[0].search_readings[0].full,
        "tongbu"
    );

    application.display_name = "音乐".to_owned();
    application.normalized_name = "音乐".to_owned();
    application.normalized_tokens = "音乐".to_owned();
    commit(&mut index, 3, &application);
    index.cache_scanned_entries(vec![application]).unwrap();
    assert_eq!(
        index.load_presentable().unwrap()[0].search_readings[0].full,
        "yinyue"
    );

    let mut application = entry("sync");
    application.display_name = "音乐".to_owned();
    application.normalized_name = "音乐".to_owned();
    application.normalized_tokens = "音乐\n同步".to_owned();
    commit(&mut index, 4, &application);
    index.cache_scanned_entries(vec![application]).unwrap();
    assert_eq!(
        index.load_presentable().unwrap()[0]
            .search_readings
            .iter()
            .map(|reading| reading.full.as_str())
            .collect::<Vec<_>>(),
        ["yinyue", "tongbu"],
        "alternate-name changes must invalidate the cached search inputs"
    );
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}

fn commit(index: &mut ApplicationIndex, generation: u64, application: &ApplicationEntry) {
    index.database.begin_scan(generation).unwrap();
    index
        .database
        .commit_scan(
            ScanReport {
                generation,
                discovered: 1,
                warnings: 0,
                complete: true,
                cancelled: false,
            },
            std::slice::from_ref(application),
            None,
        )
        .unwrap();
}

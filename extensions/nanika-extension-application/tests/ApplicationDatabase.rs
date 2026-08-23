use std::path::PathBuf;

use crate::{ApplicationArguments, ApplicationDatabase, ApplicationEntry, ScanReport};

#[test]
fn database_initializes_and_recovers_an_interrupted_scan() {
    let root = test_root("migration");
    let path = root.join("application.db");
    let database = ApplicationDatabase::open(&path).expect("database should open");
    assert_eq!(database.schema_version().expect("schema version"), 1);
    database.begin_scan(7).expect("scan should begin");
    drop(database);

    let database = ApplicationDatabase::open(&path).expect("database should reopen");
    assert_eq!(database.scan_status().expect("scan status"), "interrupted");
    database.begin_scan(8).expect("scan should begin");
    database
        .fail_scan(8, "known folders unavailable")
        .expect("scan failure should persist");
    assert_eq!(database.scan_status().expect("scan status"), "failed");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn complete_scans_stale_then_remove_missing_entries() {
    let root = test_root("stale");
    let path = root.join("application.db");
    let mut database = ApplicationDatabase::open(path).expect("database should open");
    let entry = entry("app.one", 1);
    database.begin_scan(1).expect("scan should begin");
    database
        .commit_scan(report(1, true), &[entry], None)
        .expect("first scan should commit");
    assert_eq!(database.load_active_entries().expect("entries").len(), 1);

    database.begin_scan(2).expect("scan should begin");
    database
        .commit_scan(report(2, true), &[], None)
        .expect("second scan should commit");
    assert!(database.load_active_entries().expect("entries").is_empty());

    database.begin_scan(3).expect("scan should begin");
    database
        .commit_scan(report(3, true), &[], None)
        .expect("third scan should commit");
    assert!(database.load_active_entries().expect("entries").is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn partial_scans_preserve_entries_not_seen_during_failures() {
    let root = test_root("partial");
    let path = root.join("application.db");
    let mut database = ApplicationDatabase::open(path).expect("database should open");
    database.begin_scan(1).expect("scan should begin");
    database
        .commit_scan(report(1, true), &[entry("app.one", 1)], None)
        .expect("first scan should commit");
    database.begin_scan(2).expect("scan should begin");
    database
        .commit_scan(report(2, false), &[], Some("permission denied"))
        .expect("partial scan should commit");
    assert_eq!(database.load_active_entries().expect("entries").len(), 1);
    let _ = std::fs::remove_dir_all(root);
}

fn report(generation: u64, complete: bool) -> ScanReport {
    ScanReport {
        generation,
        discovered: 0,
        warnings: usize::from(!complete),
        complete,
        cancelled: false,
    }
}

fn entry(entry_id: &str, generation: u64) -> ApplicationEntry {
    ApplicationEntry {
        entry_id: entry_id.to_owned(),
        source_key: "source".to_owned(),
        display_name: "Example".to_owned(),
        normalized_name: "example".to_owned(),
        normalized_tokens: "example".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: "example.exe".to_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: None,
        icon_key: "fallback".to_owned(),
        file_identity: "example.exe".to_owned(),
        last_seen_at: generation,
        stale: false,
        icon_source: None,
        icon_index: 0,
        priority: 0,
    }
}

fn test_root(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("nanika-application-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root should exist");
    root
}

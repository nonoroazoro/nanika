use std::io::{Seek, Write};
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
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn corrupt_generated_database_is_rebuilt() {
    let root = test_root("corrupt");
    let path = root.join("application.db");
    std::fs::write(&path, b"not a sqlite database").expect("corrupt database should exist");

    let database = ApplicationDatabase::open_recovering(&path)
        .expect("corrupt generated database should rebuild");

    assert_eq!(database.schema_version().expect("schema version"), 1);
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn corrupt_application_table_is_rebuilt() {
    let root = test_root("corrupt-table");
    let path = root.join("application.db");
    let database = ApplicationDatabase::open(&path).expect("database should open");
    drop(database);
    let connection = rusqlite::Connection::open(&path).expect("database should reopen");
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("database should checkpoint");
    let page_size = connection
        .query_row("PRAGMA page_size", [], |row| row.get::<_, i64>(0))
        .expect("page size should load") as u64;
    let root_page = connection
        .query_row(
            "SELECT rootpage FROM sqlite_schema WHERE name = 'app_entries'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("application table root should load") as u64;
    drop(connection);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("database should be writable");
    file.seek(std::io::SeekFrom::Start(
        root_page.saturating_sub(1).saturating_mul(page_size),
    ))
    .expect("table page should be seekable");
    file.write_all(&[0xff])
        .expect("table page should be corrupted");
    drop(file);

    let database = ApplicationDatabase::open_recovering(&path)
        .expect("localized corruption should rebuild the generated index");

    assert_eq!(database.schema_version().expect("schema version"), 1);
    assert!(database.load_active_entries().expect("entries").is_empty());
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn incompatible_schema_is_not_reset() {
    let root = test_root("newer-schema");
    let path = root.join("application.db");
    let database = ApplicationDatabase::open(&path).expect("database should open");
    drop(database);
    let connection = rusqlite::Connection::open(&path).expect("database should reopen");
    connection
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (2, 0)",
            [],
        )
        .expect("future migration should be recorded");
    drop(connection);

    let error = ApplicationDatabase::open_recovering(&path)
        .err()
        .expect("incompatible schema should fail");

    assert!(error.to_string().contains("newer extension"));
    assert!(path.is_file());
    std::fs::remove_dir_all(root).expect("test root should be removable");
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
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
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
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
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

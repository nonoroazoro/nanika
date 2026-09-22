use std::io::{Seek, Write};
use std::path::PathBuf;

use crate::{ApplicationArguments, ApplicationDatabase, ApplicationEntry, ScanReport};

#[test]
fn baseline_schema_is_the_only_initial_version() {
    let root = test_root("schema");
    let path = root.join("application.db");
    drop(ApplicationDatabase::open(&path).expect("database should open"));

    let connection = rusqlite::Connection::open(path).expect("database should reopen");
    let version: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("schema version should load");
    assert_eq!(version, 1);
    assert_eq!(
        table_columns(&connection, "scan_state"),
        [
            "id",
            "generation",
            "status",
            "started_at",
            "completed_at",
            "last_error",
        ]
    );
    assert_eq!(
        table_columns(&connection, "app_entries"),
        [
            "entry_id",
            "source_key",
            "display_name",
            "normalized_name",
            "normalized_tokens",
            "launch_kind",
            "target_path",
            "working_directory",
            "arguments_json",
            "bundle_id",
            "icon_key",
        ]
    );
    for table in ["scan_state", "app_entries"] {
        assert!(
            table_is_strict(&connection, table),
            "{table} must be strict"
        );
    }
    drop(connection);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn database_initializes_and_recovers_an_interrupted_scan() {
    let root = test_root("baseline");
    let path = root.join("application.db");
    let database = ApplicationDatabase::open(&path).expect("database should open");
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
fn corrupt_generated_database_fails_explicitly() {
    let root = test_root("corrupt");
    let path = root.join("application.db");
    std::fs::write(&path, b"not a sqlite database").expect("corrupt database should exist");

    assert!(ApplicationDatabase::open(&path).is_err());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn corrupt_application_table_fails_explicitly() {
    let root = test_root("corrupt-table");
    let path = root.join("application.db");
    let mut database = ApplicationDatabase::open(&path).expect("database should open");
    database.begin_scan(1).expect("scan should begin");
    database
        .commit_scan(report(1, true), &[entry("app.corrupt")], None)
        .expect("application row should persist");
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

    let result = ApplicationDatabase::open(&path)
        .and_then(|database| database.load_entries().map(|_| database));
    assert!(result.is_err());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn complete_scans_replace_the_previous_snapshot() {
    let root = test_root("snapshot");
    let path = root.join("application.db");
    let mut database = ApplicationDatabase::open(path).expect("database should open");
    let entry = entry("app.one");
    database.begin_scan(1).expect("scan should begin");
    database
        .commit_scan(report(1, true), &[entry], None)
        .expect("first scan should commit");
    assert_eq!(database.load_entries().expect("entries").len(), 1);

    database.begin_scan(2).expect("scan should begin");
    database
        .commit_scan(report(2, true), &[], None)
        .expect("second scan should commit");
    assert!(database.load_entries().expect("entries").is_empty());

    database.begin_scan(3).expect("scan should begin");
    database
        .commit_scan(report(3, true), &[], None)
        .expect("third scan should commit");
    assert!(database.load_entries().expect("entries").is_empty());
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
        .commit_scan(report(1, true), &[entry("app.one")], None)
        .expect("first scan should commit");
    database.begin_scan(2).expect("scan should begin");
    database
        .commit_scan(report(2, false), &[], Some("permission denied"))
        .expect("partial scan should commit");
    assert_eq!(database.load_entries().expect("entries").len(), 1);
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

fn entry(entry_id: &str) -> ApplicationEntry {
    ApplicationEntry {
        entry_id: entry_id.to_owned(),
        source_key: "source".to_owned(),
        display_name: "Example".to_owned(),
        normalized_name: "example".to_owned(),
        normalized_tokens: "example".to_owned(),
        search_readings: Vec::new(),
        launch_kind: "executable".to_owned(),
        target_path: "example.exe".to_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: None,
        icon_key: "fallback".to_owned(),
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

fn table_columns(connection: &rusqlite::Connection, table: &str) -> Vec<String> {
    connection
        .prepare(&format!(
            "SELECT name FROM pragma_table_info('{table}') ORDER BY cid"
        ))
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("table columns should load")
}

fn table_is_strict(connection: &rusqlite::Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT strict FROM pragma_table_list WHERE name = ?1",
            [table],
            |row| row.get(0),
        )
        .expect("table strictness should load")
}

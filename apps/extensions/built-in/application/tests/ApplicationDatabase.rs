use crate::ApplicationEntryData;
use crate::{ApplicationArguments, ApplicationDatabase, ApplicationEntry};
use std::io::{Seek, Write};
use std::path::PathBuf;

#[test]
fn schema_contains_only_persistent_application_metadata() {
    let root = test_root("schema");
    let path = root.join("application.db");
    drop(ApplicationDatabase::open(&path).unwrap());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    let tables = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(tables, ["app_sources"]);
    assert_eq!(
        table_columns(&connection, "app_sources"),
        [
            "root_key",
            "entry_id",
            "source_key",
            "display_name",
            "normalized_tokens",
            "launch_kind",
            "target_path",
            "arguments_json",
            "icon_key",
            "icon_source",
            "icon_index",
            "priority"
        ]
    );
    assert!(table_is_strict(&connection, "app_sources"));
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn root_commits_preserve_unrelated_records_and_survive_reopen() {
    let root = test_root("root-commits");
    let path = root.join("application.db");
    let mut database = ApplicationDatabase::open(&path).unwrap();
    database
        .commit_root("root", &[entry("first"), entry("unrelated")], &[])
        .unwrap();
    database
        .commit_root("root", &[entry("replacement")], &["first".to_owned()])
        .unwrap();
    drop(database);
    let database = ApplicationDatabase::open(&path).unwrap();
    let entries = database.load_entries().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|entry| entry.entry_id == "unrelated"));
    assert!(entries.iter().any(|entry| entry.entry_id == "replacement"));
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_root_upsert_rolls_back_its_deletions() {
    let root = test_root("root-rollback");
    let mut database = ApplicationDatabase::open(root.join("application.db")).unwrap();
    database
        .commit_root("root", &[entry("retained")], &[])
        .unwrap();
    let mut invalid = entry("invalid");
    invalid.display_name.clear();
    assert!(
        database
            .commit_root("root", &[invalid], &["retained".to_owned()])
            .is_err()
    );
    assert_eq!(database.load_entries().unwrap()[0].entry_id, "retained");
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
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
    database
        .commit_root("root", &[entry("app.corrupt")], &[])
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
            "SELECT rootpage FROM sqlite_schema WHERE name = 'app_sources'",
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

fn entry(entry_id: &str) -> ApplicationEntry {
    ApplicationEntry::new(ApplicationEntryData {
        entry_id: entry_id.to_owned(),
        source_key: "source".to_owned(),
        display_name: "Example".to_owned(),
        normalized_name: "example".to_owned(),
        normalized_tokens: "example".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: "example.exe".to_owned(),
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        icon_key: "fallback".to_owned(),
        icon_source: None,
        icon_index: 0,
        priority: 0,
    })
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

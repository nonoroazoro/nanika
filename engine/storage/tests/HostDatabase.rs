use crate::{ExtensionKind, HostDatabase};

#[test]
fn baseline_applies_once() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-test-{}.db", std::process::id()));
    cleanup(&database);
    let first = HostDatabase::open(&database).expect("database should open");
    drop(first);
    let second = HostDatabase::open(&database).expect("database should reopen");
    drop(second);
    cleanup(&database);
}

#[test]
fn baseline_schema_is_the_only_initial_version() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-baseline-schema-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).expect("database should open");
    drop(host);

    let connection = rusqlite::Connection::open(&database).expect("database should reopen");
    let version: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("schema version should load");
    assert_eq!(version, 1);
    assert_eq!(
        table_columns(&connection, "extensions"),
        [
            "extension_id",
            "kind",
            "version",
            "install_path",
            "package_digest",
        ]
    );
    assert_eq!(
        table_columns(&connection, "input_history"),
        ["id", "normalized_query", "display_query", "last_used_at"]
    );
    assert_eq!(
        table_columns(&connection, "usage_stats"),
        [
            "extension_id",
            "entry_id",
            "action_id",
            "query_context",
            "execution_count",
            "last_executed_at",
        ]
    );
    for table in ["extensions", "input_history", "usage_stats"] {
        assert!(
            table_is_strict(&connection, table),
            "{table} must be strict"
        );
    }
    let history_ordering_index: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_schema
                WHERE type = 'index' AND name = 'input_history_last_used'
             )",
            [],
            |row| row.get(0),
        )
        .expect("history ordering index should load");
    assert!(history_ordering_index);
    let migration_table_exists: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_schema
                WHERE type = 'table' AND name LIKE '%migration%'
             )",
            [],
            |row| row.get(0),
        )
        .expect("migration table check should load");
    assert!(!migration_table_exists);
    drop(connection);
    cleanup(&database);
}

#[test]
fn external_package_metadata_round_trips_without_affecting_builtins() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-extension-state-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).expect("database should open");
    host.register_builtin_extension("com.nanika.command")
        .expect("built-in should register");
    host.install_external_extension(
        "com.example.extension",
        "1.2.3",
        std::path::Path::new("C:/nanika/extensions/com.example.extension/1.2.3"),
        "digest",
    )
    .expect("external extension should install");

    let installed = host
        .extension("com.example.extension")
        .expect("extension should load")
        .expect("extension should exist");
    assert_eq!(installed.kind, ExtensionKind::External);
    assert_eq!(installed.version.as_deref(), Some("1.2.3"));
    assert!(
        host.remove_external_extension("com.example.extension")
            .expect("external extension should remove")
    );
    assert!(
        host.extension("com.nanika.command")
            .expect("built-in should load")
            .is_some()
    );
    drop(host);
    cleanup(&database);
}

#[test]
fn obsolete_schema_is_rejected_without_rewriting_user_records() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-obsolete-{}.db", std::process::id()));
    cleanup(&database);
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch("CREATE TABLE extensions (extension_id TEXT, state TEXT); INSERT INTO extensions VALUES ('com.example.saved', 'disabled');").unwrap();
    let error = HostDatabase::open(&database)
        .err()
        .expect("obsolete schema must fail explicitly");
    assert!(error.to_string().contains("unsupported database schema"));
    let saved: String = connection
        .query_row(
            "SELECT state FROM extensions WHERE extension_id = 'com.example.saved'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(saved, "disabled");
    drop(connection);
    cleanup(&database);
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

fn cleanup(database: &std::path::Path) {
    let _ = std::fs::remove_file(database);
    let _ = std::fs::remove_file(database.with_extension("db-wal"));
    let _ = std::fs::remove_file(database.with_extension("db-shm"));
}
#[test]
fn repeated_builtin_registration_does_not_write_inventory() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-idempotent-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).unwrap();
    host.register_builtin_extension("com.nanika.command")
        .unwrap();
    let observer = rusqlite::Connection::open(&database).unwrap();
    observer.execute_batch("CREATE TRIGGER reject_rewrite BEFORE UPDATE ON extensions BEGIN SELECT RAISE(ABORT, 'unchanged inventory must not be rewritten'); END;").unwrap();
    host.register_builtin_extension("com.nanika.command")
        .unwrap();
    assert!(host.extension("com.nanika.command").unwrap().is_some());
    drop(observer);
    drop(host);
    cleanup(&database);
}

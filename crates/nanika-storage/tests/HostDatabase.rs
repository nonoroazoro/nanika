use crate::{ExtensionKind, HostDatabase};

#[test]
fn baseline_applies_once() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-test-{}.db", std::process::id()));
    cleanup(&database);
    let first = HostDatabase::open(&database).expect("database should open");
    assert_eq!(first.schema_version().expect("schema version"), 1);
    drop(first);
    let second = HostDatabase::open(&database).expect("database should reopen");
    assert_eq!(second.schema_version().expect("schema version"), 1);
    drop(second);
    cleanup(&database);
}

#[test]
fn baseline_schema_contains_entry_identity_and_retention_index() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-baseline-schema-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).expect("database should open");
    assert_eq!(host.schema_version().expect("schema version"), 1);
    drop(host);

    let connection = rusqlite::Connection::open(&database).expect("database should reopen");
    let columns = connection
        .prepare("SELECT name FROM pragma_table_info('usage_stats') ORDER BY cid")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("usage columns should load");
    assert_eq!(
        columns,
        [
            "extension_id",
            "entry_id",
            "action_id",
            "query_context",
            "execution_count",
            "last_executed_at",
        ]
    );
    let retention_index: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_schema
                WHERE type = 'index' AND name = 'usage_stats_last_executed_at'
             )",
            [],
            |row| row.get(0),
        )
        .expect("retention index should load");
    assert!(retention_index);
    drop(connection);
    cleanup(&database);
}

#[test]
fn unsupported_migration_history_is_rejected() {
    for (suffix, versions) in [("newer", vec![2_i64]), ("unexpected", vec![0_i64])] {
        let database =
            std::env::temp_dir().join(format!("nanika-storage-{suffix}-{}.db", std::process::id()));
        cleanup(&database);
        let connection = rusqlite::Connection::open(&database).expect("database should open");
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at INTEGER NOT NULL
                 );",
            )
            .expect("migration table should exist");
        for version in versions {
            connection
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, 1)",
                    [version],
                )
                .expect("version should be inserted");
        }
        drop(connection);
        assert!(HostDatabase::open(&database).is_err());
        cleanup(&database);
    }
}

#[test]
fn external_installation_state_round_trips_without_affecting_builtins() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-extension-state-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).expect("database should open");
    host.register_extension("com.nanika.command", ExtensionKind::BuiltIn, 1)
        .expect("built-in should register");
    host.install_external_extension(
        "com.example.extension",
        "1.2.3",
        std::path::Path::new("C:/nanika/extensions/com.example.extension/1.2.3"),
        "digest",
        true,
        2,
    )
    .expect("external extension should install");

    let installed = host
        .extension("com.example.extension")
        .expect("extension should load")
        .expect("extension should exist");
    assert_eq!(installed.kind, ExtensionKind::External);
    assert_eq!(installed.active_version.as_deref(), Some("1.2.3"));
    assert!(
        host.set_external_extension_enabled("com.example.extension", false, 3)
            .expect("extension should disable")
    );
    assert_eq!(
        host.extension("com.example.extension")
            .expect("extension should load")
            .expect("extension should exist")
            .state,
        "disabled"
    );
    assert!(
        !host
            .set_external_extension_enabled("com.nanika.command", false, 4)
            .expect("built-in must not mutate through external API")
    );
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

fn cleanup(database: &std::path::Path) {
    let _ = std::fs::remove_file(database);
    let _ = std::fs::remove_file(database.with_extension("db-wal"));
    let _ = std::fs::remove_file(database.with_extension("db-shm"));
}

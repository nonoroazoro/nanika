use crate::{ExtensionKind, HostDatabase, migrations::MIGRATIONS};

#[test]
fn migrations_apply_once() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-test-{}.db", std::process::id()));
    cleanup(&database);
    let first = HostDatabase::open(&database).expect("database should open");
    assert_eq!(first.schema_version().expect("schema version"), 3);
    drop(first);
    let second = HostDatabase::open(&database).expect("database should reopen");
    assert_eq!(second.schema_version().expect("schema version"), 3);
    drop(second);
    cleanup(&database);
}

#[test]
fn usage_schema_migrates_entry_identity_forward() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-usage-migration-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let legacy = rusqlite::Connection::open(&database).expect("legacy database should open");
    legacy
        .execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
             );",
        )
        .expect("migration table should exist");
    legacy
        .execute_batch(MIGRATIONS[0].1)
        .expect("legacy schema should exist");
    legacy
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (1, 1)",
            [],
        )
        .expect("legacy version should exist");
    legacy
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, state, health, updated_at
             ) VALUES ('test.extension', 'external', 'enabled', 'healthy', 1)",
            [],
        )
        .expect("legacy extension should exist");
    legacy
        .execute(
            "INSERT INTO usage_stats (
                extension_id, action_id, query_context, execution_count, last_executed_at
             ) VALUES ('test.extension', 'open', 'tool', 2, 10)",
            [],
        )
        .expect("legacy usage should exist");
    drop(legacy);

    let migrated = HostDatabase::open(&database).expect("database should migrate");
    assert_eq!(migrated.schema_version().expect("schema version"), 3);
    let usage = migrated.load_usage().expect("usage should migrate");
    assert_eq!(usage[0].entry_id, "open");
    assert_eq!(usage[0].execution_count, 2);
    drop(migrated);
    cleanup(&database);
}

#[test]
fn newer_or_gapped_migration_history_is_rejected() {
    for (suffix, versions) in [("newer", vec![99_i64]), ("gapped", vec![1_i64, 3_i64])] {
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

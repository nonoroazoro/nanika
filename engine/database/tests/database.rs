use super::open;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const SCHEMA: &str = "CREATE TABLE entries (id TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT; CREATE INDEX entries_value ON entries(value); PRAGMA user_version=1;";

#[test]
fn initializes_once_and_preserves_records() {
    let path = _path();
    let database = open(&path, SCHEMA).unwrap();
    database
        .execute("INSERT INTO entries VALUES ('saved', 'content')", [])
        .unwrap();
    drop(database);
    let database = open(&path, SCHEMA).unwrap();
    assert_eq!(
        database
            .query_row("SELECT value FROM entries", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "content"
    );
    assert_eq!(
        database
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    drop(database);
    _cleanup(path);
}

#[test]
fn rejects_every_schema_object_change_without_rewriting_data() {
    for mutation in [
        "CREATE TABLE obsolete (value TEXT)",
        "CREATE TABLE sqliteXobsolete (value TEXT)",
        "ALTER TABLE entries ADD COLUMN obsolete TEXT",
        "DROP INDEX entries_value",
        "CREATE INDEX obsolete ON entries(id)",
        "CREATE TRIGGER obsolete AFTER INSERT ON entries BEGIN SELECT 1; END",
        "PRAGMA user_version=2",
    ] {
        let path = _path();
        let database = open(&path, SCHEMA).unwrap();
        database
            .execute("INSERT INTO entries VALUES ('saved', 'content')", [])
            .unwrap();
        database.execute_batch(mutation).unwrap();
        let before = super::_objects(&database).unwrap();
        assert!(
            open(&path, SCHEMA)
                .unwrap_err()
                .to_string()
                .contains("unsupported database schema")
        );
        assert_eq!(super::_objects(&database).unwrap(), before);
        assert_eq!(
            database
                .query_row("SELECT value FROM entries", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "content"
        );
        drop(database);
        _cleanup(path);
    }
}

#[test]
fn rejects_corruption_and_incomplete_schema_instead_of_repairing() {
    let path = _path();
    let database = Connection::open(&path).unwrap();
    database.execute_batch("CREATE TABLE entries (id TEXT, value TEXT); INSERT INTO entries VALUES ('saved', 'content');").unwrap();
    assert!(open(&path, SCHEMA).is_err());
    assert_eq!(
        database
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        0
    );
    drop(database);
    _cleanup(path);
    let path = _path();
    std::fs::write(&path, b"invalid database").unwrap();
    assert!(open(&path, SCHEMA).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"invalid database");
    _cleanup(path);
}

fn _path() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "nanika-database-{}-{}.db",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn _cleanup(path: PathBuf) {
    for file in [
        path.clone(),
        path.with_extension("db-wal"),
        path.with_extension("db-shm"),
    ] {
        if file.exists() {
            std::fs::remove_file(file).unwrap();
        }
    }
}

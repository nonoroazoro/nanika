use super::*;
use crate::{ClipboardDatabase, ClipboardImageDimensions};
use nanika_protocol::ClipboardContent;

#[test]
fn migration_uses_payload_facts_and_preserves_raw_data_and_order() {
    let root = _root("preserved");
    let path = root.join("clipboard.db");
    let payloads = root.join("payloads");
    let image = payloads.join("old.png");
    let bytes = include_bytes!("../assets/icon.png");
    std::fs::write(&image, bytes).unwrap();
    let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    let text = "\n Image 999 x 888\n用户原文\u{0}unchanged";
    let files = "[ \"/old/name and 99 more\", \"/old/另一份.txt\" ]";
    connection.execute("INSERT INTO clipboard_entries VALUES ('text', 'text', 'Wrong English title', ?1, NULL, NULL, 900, 101)", [text]).unwrap();
    connection.execute("INSERT INTO clipboard_entries VALUES ('files', 'files', 'Not a filename and 999 more', NULL, ?1, NULL, 901, 102)", [files]).unwrap();
    connection.execute("INSERT INTO clipboard_entries VALUES ('image', 'image', 'Image 999 x 888', NULL, NULL, ?1, 902, 103)", [image.to_str().unwrap()]).unwrap();
    let before = _payload_rows(&connection);
    drop(connection);

    let database = ClipboardDatabase::open(&path, &payloads).unwrap();
    assert_eq!(
        database.matching_ids("", "all").unwrap(),
        ["image", "files", "text"]
    );
    let entry = database.entry("image").unwrap();
    let (width, height) = nanika_platform::png_resource_dimensions(bytes).unwrap();
    assert_eq!(
        entry.image_dimensions,
        Some(ClipboardImageDimensions { width, height })
    );
    assert_ne!((width, height), (999, 888));
    assert_eq!(
        database.entry("text").unwrap().content,
        ClipboardContent::Text { value: text.into() }
    );
    let row = database.items(&["files".into()]).unwrap().remove(0);
    let nanika_protocol::LocalizedText::Message(title) = row.summary.title().unwrap() else {
        panic!("file collection reference")
    };
    assert_eq!(title.args["remaining"], 1i64.into());
    assert_eq!(title.args["name"], "name and 99 more".into());
    drop(database);
    let connection = nanika_database::open(&path, crate::clipboard_database::SCHEMA).unwrap();
    assert_eq!(_payload_rows(&connection), before);
    assert!(
        connection
            .prepare("SELECT title FROM clipboard_entries")
            .is_err()
    );
    assert_eq!(std::fs::read(&image).unwrap(), bytes);
    drop(connection);
    // Metadata is durable: opening, searching, selecting and formatting do not reopen PNGs.
    std::fs::remove_file(image).unwrap();
    let database = ClipboardDatabase::open(&path, &payloads).unwrap();
    for query in ["image", "图片", &format!("{width} x {height}")] {
        assert_eq!(database.matching_ids(query, "images").unwrap(), ["image"]);
    }
    assert_eq!(database.entry("image").unwrap(), entry);
    assert!(
        database.items(&["image".into()]).unwrap()[0]
            .summary
            .title()
            .is_ok()
    );
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_or_invalid_metadata_rolls_back_and_retries_after_payload_repair() {
    for content in [None, Some(b"invalid PNG".as_slice())] {
        let root = _root(if content.is_some() {
            "corrupt"
        } else {
            "missing"
        });
        let path = root.join("clipboard.db");
        let payloads = root.join("payloads");
        let image = payloads.join("old.png");
        if let Some(bytes) = content {
            std::fs::write(&image, bytes).unwrap();
        }
        let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
        connection.execute("INSERT INTO clipboard_entries VALUES ('a-text', 'text', 'Original title', 'original payload', NULL, NULL, 16, 10)", []).unwrap();
        connection.execute("INSERT INTO clipboard_entries VALUES ('z-image', 'image', 'Untrusted dimensions', NULL, NULL, ?1, 90, 11)", [image.to_str().unwrap()]).unwrap();
        let before = _payload_rows(&connection);
        drop(connection);
        let error = ClipboardDatabase::open(&path, &payloads).err().unwrap();
        assert!(
            error.contains("z-image") && error.contains(image.to_str().unwrap()),
            "{error}"
        );
        assert!(error.contains("preserved") && error.contains("retry"));
        let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
        assert_eq!(_payload_rows(&connection), before);
        assert_eq!(
            connection
                .query_row(
                    "SELECT title FROM clipboard_entries WHERE entry_id = 'a-text'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "Original title"
        );
        drop(connection);
        std::fs::write(&image, include_bytes!("../assets/icon.png")).unwrap();
        drop(ClipboardDatabase::open(&path, &payloads).unwrap());
        let connection = nanika_database::open(&path, crate::clipboard_database::SCHEMA).unwrap();
        assert_eq!(_payload_rows(&connection), before);
        drop(connection);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn unknown_schema_and_version_preserve_related_records() {
    for extra in [
        "CREATE TABLE related (entry_id TEXT, usage INTEGER); INSERT INTO related VALUES ('kept', 42);",
        "PRAGMA user_version = 2;"
    ] {
        let root = _root(if extra.starts_with("CREATE") {
            "related"
        } else {
            "version"
        });
        let path = root.join("clipboard.db");
        let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
        connection.execute("INSERT INTO clipboard_entries VALUES ('kept', 'text', 'Title', 'payload', NULL, NULL, 7, 20)", []).unwrap();
        connection.execute_batch(extra).unwrap();
        let schema = _schema(&connection).unwrap();
        let rows = _payload_rows(&connection);
        assert!(ClipboardDatabase::open(&path, &root.join("payloads")).is_err());
        assert_eq!(_schema(&connection).unwrap(), schema);
        assert_eq!(_payload_rows(&connection), rows);
        if extra.starts_with("CREATE") {
            assert_eq!(
                connection
                    .query_row(
                        "SELECT usage FROM related WHERE entry_id = 'kept'",
                        [],
                        |row| row.get::<_, i64>(0)
                    )
                    .unwrap(),
                42
            );
        }
        drop(connection);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn interrupted_migration_rolls_back_schema_and_rows_and_can_retry() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering}
    };
    let root = _root("interrupt");
    let path = root.join("clipboard.db");
    let mut connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    connection.execute_batch("WITH RECURSIVE sequence(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM sequence WHERE n<1000) INSERT INTO clipboard_entries SELECT printf('%04d', n), 'text', 'title', 'payload', NULL, NULL, 7, n FROM sequence;").unwrap();
    let before = _payload_rows(&connection);
    let inserted = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&inserted);
    connection
        .update_hook(Some(move |action, _: &str, table: &str, _| {
            if action == rusqlite::hooks::Action::SQLITE_INSERT && table == "clipboard_entries" {
                observed.fetch_add(1, Ordering::SeqCst);
            }
        }))
        .unwrap();
    let observed = Arc::clone(&inserted);
    connection
        .progress_handler(1, Some(move || observed.load(Ordering::SeqCst) >= 5))
        .unwrap();
    let error = _migrate(&mut connection, &root.join("payloads")).unwrap_err();
    assert!(error.contains("interrupted"), "{error}");
    assert_eq!(inserted.load(Ordering::SeqCst), 5);
    connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    assert_eq!(_payload_rows(&connection), before);
    let expected = Connection::open_in_memory().unwrap();
    expected.execute_batch(LEGACY_SCHEMA).unwrap();
    assert_eq!(_schema(&connection).unwrap(), _schema(&expected).unwrap());
    drop(connection);
    drop(ClipboardDatabase::open(&path, &root.join("payloads")).unwrap());
    let connection = nanika_database::open(&path, crate::clipboard_database::SCHEMA).unwrap();
    assert_eq!(_payload_rows(&connection), before);
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn migration_rejects_payload_symlinks_outside_the_owned_root() {
    let root = _root("symlink");
    let path = root.join("clipboard.db");
    let outside = root.join("outside.png");
    std::fs::write(&outside, include_bytes!("../assets/icon.png")).unwrap();
    let link = root.join("payloads/image.png");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    connection.execute("INSERT INTO clipboard_entries VALUES ('image', 'image', 'Image', NULL, NULL, ?1, 1, 1)", [link.to_str().unwrap()]).unwrap();
    let rows = _payload_rows(&connection);
    drop(connection);
    assert!(ClipboardDatabase::open(&path, &root.join("payloads")).is_err());
    let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    assert_eq!(_payload_rows(&connection), rows);
    assert_eq!(
        std::fs::read(outside).unwrap(),
        include_bytes!("../assets/icon.png")
    );
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_writer_prevents_migration_without_changing_history() {
    let root = _root("locked");
    let path = root.join("clipboard.db");
    let mut writer = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    writer.execute("INSERT INTO clipboard_entries VALUES ('kept', 'text', 'Title', 'payload', NULL, NULL, 7, 20)", []).unwrap();
    let before = _payload_rows(&writer);
    let transaction = writer
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let error = ClipboardDatabase::open(&path, &root.join("payloads"))
        .err()
        .unwrap();
    assert!(error.contains("locked"), "{error}");
    assert_eq!(_payload_rows(&transaction), before);
    transaction.rollback().unwrap();
    drop(writer);
    drop(ClipboardDatabase::open(&path, &root.join("payloads")).unwrap());
    let connection = nanika_database::open(&path, crate::clipboard_database::SCHEMA).unwrap();
    assert_eq!(_payload_rows(&connection), before);
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}

fn _root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-migration-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("payloads")).unwrap();
    root
}

fn _payload_rows(connection: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
    connection.prepare("SELECT entry_id, content_kind, text_payload, files_json, image_path, byte_size, captured_at FROM clipboard_entries ORDER BY entry_id").unwrap().query_map([], |row| (0..7).map(|i| row.get(i)).collect()).unwrap().collect::<Result<_, _>>().unwrap()
}

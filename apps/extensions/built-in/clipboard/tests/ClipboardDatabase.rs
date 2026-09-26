use nanika_protocol::ClipboardContent;

use crate::{ClipboardConfig, ClipboardDatabase, ClipboardEntry};

#[test]
fn baseline_schema_is_the_only_initial_version() {
    let root = std::env::temp_dir().join(format!("nanika-clipboard-schema-{}", std::process::id()));
    let path = root.join("clipboard.db");
    drop(ClipboardDatabase::open(&path).expect("database"));

    let connection = rusqlite::Connection::open(path).expect("database should reopen");
    let version: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("schema version should load");
    assert_eq!(version, 1);
    let columns = connection
        .prepare("SELECT name FROM pragma_table_info('clipboard_entries') ORDER BY cid")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .expect("clipboard columns should load");
    assert_eq!(
        columns,
        [
            "entry_id",
            "content_kind",
            "title",
            "text_payload",
            "files_json",
            "image_path",
            "byte_size",
            "captured_at",
        ]
    );
    let is_strict: bool = connection
        .query_row(
            "SELECT strict FROM pragma_table_list WHERE name = 'clipboard_entries'",
            [],
            |row| row.get(0),
        )
        .expect("table strictness should load");
    assert!(is_strict);
    let error = connection
        .execute(
            "INSERT INTO clipboard_entries (
                entry_id, content_kind, title, text_payload, files_json,
                image_path, byte_size, captured_at
             ) VALUES ('invalid', 'text', 'invalid', 'text', '[]', NULL, 1, 1)",
            [],
        )
        .expect_err("mixed payload columns must be rejected");
    assert!(error.to_string().contains("CHECK constraint failed"));
    drop(connection);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn clipboard_database_initializes_deduplicates_and_loads_content() {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-database-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    let mut entry = ClipboardEntry {
        entry_id: "clipboard.hash".to_owned(),
        title: "first".to_owned(),
        content: ClipboardContent::Text {
            value: "payload".to_owned(),
        },
        byte_size: 7,
        captured_at: 10,
    };
    database.upsert(&entry).expect("first capture");
    entry.title = "second".to_owned();
    entry.captured_at = 20;
    database.upsert(&entry).expect("duplicate capture");
    let loaded = database.load().expect("history");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].title, "second");
    assert_eq!(loaded[0].captured_at, 20);
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn retention_removes_entries_outside_the_count_limit() {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-storage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    for index in 0..3 {
        database
            .upsert(&text_entry(index, 10_000 + index))
            .expect("capture");
    }
    database
        .apply_retention(
            20_000,
            &ClipboardConfig {
                max_entries: Some(2),
                max_age_days: Some(7),
            },
        )
        .expect("retention");
    let loaded = database.load().expect("history");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].entry_id, "clipboard.2");
    assert_eq!(loaded[1].entry_id, "clipboard.1");
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn retention_removes_entries_older_than_the_age_limit() {
    const DAY: u64 = 24 * 60 * 60 * 1_000;
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-age-retention-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    database.upsert(&text_entry(1, DAY)).expect("old capture");
    database
        .upsert(&text_entry(2, 8 * DAY))
        .expect("recent capture");

    database
        .apply_retention(
            8 * DAY,
            &ClipboardConfig {
                max_entries: Some(50),
                max_age_days: Some(7),
            },
        )
        .expect("retention");

    let loaded = database.load().expect("history");
    assert_eq!(loaded.len(), 2, "the cutoff itself remains retained");
    database
        .apply_retention(
            8 * DAY + 1,
            &ClipboardConfig {
                max_entries: Some(50),
                max_age_days: Some(7),
            },
        )
        .expect("retention after cutoff");
    assert_eq!(database.load().expect("history").len(), 1);
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn clear_removes_requested_clipboard_history() {
    let root = std::env::temp_dir().join(format!("nanika-clipboard-clear-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    database
        .upsert(&ClipboardEntry {
            entry_id: "clipboard.one".to_owned(),
            title: "one".to_owned(),
            content: ClipboardContent::Text {
                value: "one".to_owned(),
            },
            byte_size: 3,
            captured_at: 1,
        })
        .expect("capture");
    database
        .clear(&["clipboard.one".to_owned()])
        .expect("clear history");
    assert!(database.load().expect("history").is_empty());
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

fn text_entry(index: u64, captured_at: u64) -> ClipboardEntry {
    ClipboardEntry {
        entry_id: format!("clipboard.{index}"),
        title: format!("entry {index}"),
        content: ClipboardContent::Text {
            value: format!("entry {index}"),
        },
        byte_size: 7,
        captured_at,
    }
}

#[test]
fn unlimited_retention_and_independent_limits_preserve_the_requested_history() {
    const DAY: u64 = 24 * 60 * 60 * 1_000;
    for (name, config, expected) in [
        (
            "unlimited",
            ClipboardConfig {
                max_entries: None,
                max_age_days: None,
            },
            vec![3, 2, 1],
        ),
        (
            "count",
            ClipboardConfig {
                max_entries: Some(2),
                max_age_days: None,
            },
            vec![3, 2],
        ),
        (
            "age",
            ClipboardConfig {
                max_entries: None,
                max_age_days: Some(1),
            },
            vec![3],
        ),
    ] {
        let root =
            std::env::temp_dir().join(format!("nanika-clipboard-{name}-{}", std::process::id()));
        let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
        for (index, captured_at) in [(1, DAY), (2, 2 * DAY), (3, 10 * DAY)] {
            database
                .upsert(&text_entry(index, captured_at))
                .expect("capture");
        }
        database
            .apply_retention(10 * DAY, &config)
            .expect("requested retention");
        let ids: Vec<_> = database
            .load()
            .expect("history")
            .iter()
            .map(|entry| entry.entry_id.clone())
            .collect();
        assert_eq!(
            ids,
            expected
                .into_iter()
                .map(|index| format!("clipboard.{index}"))
                .collect::<Vec<_>>()
        );
        database
            .apply_retention(
                10 * DAY,
                &ClipboardConfig {
                    max_entries: Some(1),
                    max_age_days: Some(1),
                },
            )
            .expect("reenabled limits");
        assert_eq!(database.load().expect("history").len(), 1);
        drop(database);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}

#[test]
fn clear_preserves_unmatched_entries_and_retained_image_paths() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-scoped-clear-{}",
        std::process::id()
    ));
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    let first = text_entry(1, 1);
    let second = text_entry(2, 2);
    let mut image = text_entry(3, 3);
    let image_path = root.join("retained.png");
    image.content = ClipboardContent::PngFile {
        path: image_path.to_string_lossy().into_owned(),
    };
    for entry in [&first, &second, &image] {
        database.upsert(entry).expect("capture");
    }
    database.clear(&[]).expect("empty scope");
    assert_eq!(database.load().expect("history").len(), 3);
    let retained = database
        .clear(&[first.entry_id.clone(), "missing".to_owned()])
        .expect("clear one");
    assert_eq!(
        retained.retained_images,
        std::collections::HashSet::from([image_path])
    );
    let loaded = database.load().expect("history");
    assert_eq!(
        loaded
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>(),
        ["clipboard.3", "clipboard.2"]
    );
    database.clear(&[image.entry_id]).expect("clear image");
    assert_eq!(
        database.load().expect("history")[0].entry_id,
        second.entry_id
    );
    drop(database);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn clear_rolls_back_the_entire_scope_on_database_failure() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-clear-rollback-{}",
        std::process::id()
    ));
    let path = root.join("clipboard.db");
    let database = ClipboardDatabase::open(&path).expect("database");
    for index in 1..=2 {
        database.upsert(&text_entry(index, index)).expect("capture");
    }
    let connection = rusqlite::Connection::open(&path).expect("test connection");
    connection.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON clipboard_entries WHEN OLD.entry_id = 'clipboard.2' BEGIN SELECT RAISE(ABORT, 'test clear failure'); END;").expect("failure trigger");
    let error = database
        .clear(&["clipboard.1".to_owned(), "clipboard.2".to_owned()])
        .expect_err("clear must fail");
    assert!(error.contains("test clear failure"));
    assert_eq!(database.load().expect("history").len(), 2);
    drop(connection);
    drop(database);
    std::fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn committed_changes_match_database_order_and_keep_untouched_payloads() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-incremental-{}",
        std::process::id()
    ));
    let database = ClipboardDatabase::open(root.join("clipboard.db")).unwrap();
    let config = ClipboardConfig {
        max_entries: Some(3),
        max_age_days: None,
    };
    let mut entries = Vec::new();
    for (id, time) in [(1, 10), (2, 20), (3, 20)] {
        let entry = text_entry(id, time);
        let change = database
            .upsert_with_retention(&entry, time, &config)
            .unwrap();
        change.apply(&mut entries, Some(entry));
        assert_eq!(entries, database.load().unwrap());
    }
    let untouched = |entries: &[ClipboardEntry]| match &entries
        .iter()
        .find(|entry| entry.entry_id == "clipboard.2")
        .unwrap()
        .content
    {
        ClipboardContent::Text { value } => value.as_ptr() as usize,
        _ => unreachable!(),
    };
    let pointer = untouched(&entries);
    for (id, time) in [(1, 30), (4, 40), (2, 50), (5, 1)] {
        let entry = text_entry(id, time);
        let change = database
            .upsert_with_retention(&entry, time, &config)
            .unwrap();
        change.apply(&mut entries, Some(entry));
        assert_eq!(entries, database.load().unwrap());
        if id == 1 || id == 4 {
            assert_eq!(untouched(&entries), pointer);
        }
    }
    let change = database
        .clear(&["clipboard.2".into(), "missing".into()])
        .unwrap();
    assert_eq!(
        change.removed,
        std::collections::HashSet::from(["clipboard.2".to_owned()])
    );
    change.apply(&mut entries, None);
    assert_eq!(entries, database.load().unwrap());
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

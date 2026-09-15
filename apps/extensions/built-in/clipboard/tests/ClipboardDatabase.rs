use nanika_protocol::ClipboardContent;

use crate::{ClipboardConfig, ClipboardDatabase, ClipboardEntry};

#[test]
fn clipboard_database_initializes_deduplicates_and_loads_content() {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-database-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    let mut entry = ClipboardEntry {
        entry_id: "clipboard.hash".to_owned(),
        content_hash: "hash".to_owned(),
        title: "first".to_owned(),
        content: ClipboardContent::Text {
            value: "payload".to_owned(),
        },
        byte_size: 7,
        captured_at: 10,
        pinned: false,
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
                max_entries: 2,
                max_age_days: 7,
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
                max_entries: 50,
                max_age_days: 7,
            },
        )
        .expect("retention");

    let loaded = database.load().expect("history");
    assert_eq!(loaded.len(), 2, "the cutoff itself remains retained");
    database
        .apply_retention(
            8 * DAY + 1,
            &ClipboardConfig {
                max_entries: 50,
                max_age_days: 7,
            },
        )
        .expect("retention after cutoff");
    assert_eq!(database.load().expect("history").len(), 1);
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn clear_removes_all_clipboard_history() {
    let root = std::env::temp_dir().join(format!("nanika-clipboard-clear-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    database
        .upsert(&ClipboardEntry {
            entry_id: "clipboard.one".to_owned(),
            content_hash: "one".to_owned(),
            title: "one".to_owned(),
            content: ClipboardContent::Text {
                value: "one".to_owned(),
            },
            byte_size: 3,
            captured_at: 1,
            pinned: false,
        })
        .expect("capture");
    database.clear().expect("clear history");
    assert!(database.load().expect("history").is_empty());
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

fn text_entry(index: u64, captured_at: u64) -> ClipboardEntry {
    ClipboardEntry {
        entry_id: format!("clipboard.{index}"),
        content_hash: index.to_string(),
        title: format!("entry {index}"),
        content: ClipboardContent::Text {
            value: format!("entry {index}"),
        },
        byte_size: 7,
        captured_at,
        pinned: false,
    }
}

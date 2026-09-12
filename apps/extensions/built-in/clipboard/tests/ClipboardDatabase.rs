use nanika_protocol::ClipboardContent;

use crate::{ClipboardDatabase, ClipboardEntry};

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
fn clipboard_history_is_not_deleted_without_an_explicit_user_action() {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-storage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    database
        .upsert(&ClipboardEntry {
            entry_id: "clipboard.old".to_owned(),
            content_hash: "old".to_owned(),
            title: "old".to_owned(),
            content: ClipboardContent::Text {
                value: "old".to_owned(),
            },
            byte_size: 3,
            captured_at: 1,
            pinned: false,
        })
        .expect("old capture");
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

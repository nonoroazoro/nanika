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
fn clipboard_retention_removes_expired_unpinned_entries() {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-retention-{}", std::process::id()));
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
    database.prune(31 * 86_400).expect("retention");
    assert!(database.load().expect("history").is_empty());
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn clipboard_payload_cleanup_removes_only_unreferenced_managed_files() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-payload-cleanup-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let payload_root = root.join("payloads");
    std::fs::create_dir_all(&payload_root).expect("payload root");
    let referenced = payload_root.join(format!("{}.png", "a".repeat(64)));
    let orphan = payload_root.join(format!("{}.png", "b".repeat(64)));
    let temporary = payload_root.join(format!("{}.tmp-1", "c".repeat(64)));
    let unrelated_png = payload_root.join("keep.png");
    let unrelated_temporary = payload_root.join(format!("{}.tmp-active", "d".repeat(64)));
    for path in [
        &referenced,
        &orphan,
        &temporary,
        &unrelated_png,
        &unrelated_temporary,
    ] {
        std::fs::write(path, b"payload").expect("payload should write");
    }
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    database
        .upsert(&ClipboardEntry {
            entry_id: "clipboard.image".to_owned(),
            content_hash: "image".to_owned(),
            title: "image".to_owned(),
            content: ClipboardContent::PngFile {
                path: referenced.to_string_lossy().into_owned(),
            },
            byte_size: 7,
            captured_at: 1,
            pinned: false,
        })
        .expect("image should persist");
    database
        .cleanup_payloads(&payload_root)
        .expect("payload cleanup");
    assert!(referenced.is_file());
    assert!(!orphan.exists());
    assert!(!temporary.exists());
    assert!(unrelated_png.is_file());
    assert!(unrelated_temporary.is_file());
    drop(database);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

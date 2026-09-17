use super::clear_entries;
use crate::{ClipboardDatabase, ClipboardEntry};
use nanika_protocol::ClipboardContent;
use std::sync::RwLock;

#[test]
fn scoped_clear_preserves_other_payloads_and_publishes_committed_state() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-clear-payloads-{}",
        std::process::id()
    ));
    let payloads = root.join("payloads");
    std::fs::create_dir_all(&payloads).expect("payload root");
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    let paths = [
        payloads.join(format!("{}.png", "a".repeat(64))),
        payloads.join(format!("{}.png", "b".repeat(64))),
    ];
    for (index, path) in paths.iter().enumerate() {
        std::fs::write(path, b"payload").expect("payload");
        database
            .upsert(&ClipboardEntry {
                entry_id: index.to_string(),
                content_hash: index.to_string(),
                title: "Image".to_owned(),
                content: ClipboardContent::PngFile {
                    path: path.to_string_lossy().into_owned(),
                },
                byte_size: 7,
                captured_at: index as u64,
            })
            .expect("capture");
    }
    let entries = RwLock::new(database.load().expect("history"));
    clear_entries(&database, &payloads, &entries, &["0".to_owned()]).expect("clear first image");
    assert!(!paths[0].exists());
    assert!(paths[1].exists());
    assert_eq!(entries.read().expect("entries")[0].entry_id, "1");
    assert_eq!(entries.read().expect("entries").len(), 1);

    let invalid_root = root.join("not-a-directory");
    std::fs::write(&invalid_root, b"not a directory").expect("failure fixture");
    clear_entries(&database, &invalid_root, &entries, &["1".to_owned()])
        .expect_err("cleanup failure must be reported");
    assert!(database.load().expect("history").is_empty());
    assert!(
        entries.read().expect("entries").is_empty(),
        "memory must reflect committed deletion despite cleanup failure"
    );
    drop(database);
    std::fs::remove_dir_all(root).expect("cleanup");
}

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

#[test]
fn failed_capture_does_not_block_later_capture_or_poison_shutdown() {
    use crate::{ClipboardCommand, ClipboardConfig, ClipboardWorker};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let root = std::env::temp_dir().join(format!("nanika-capture-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let entries = Arc::new(RwLock::new(Vec::new()));
    let invalidations = Arc::new(AtomicUsize::new(0));
    let changed = Arc::clone(&invalidations);
    let mut attempt = 0;
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            max_entries: None,
            max_age_days: None,
        },
        Arc::clone(&entries),
        Arc::new(move || {
            changed.fetch_add(1, Ordering::SeqCst);
        }),
        move |_| {
            attempt += 1;
            if attempt != 2 {
                return Err("clipboard text exceeds capture limit".into());
            }
            Ok(Some(ClipboardEntry {
                entry_id: "next-copy".into(),
                content_hash: "next-copy".into(),
                title: "Next copy".into(),
                content: ClipboardContent::Text {
                    value: "Next copy".into(),
                },
                byte_size: 9,
                captured_at: 1,
            }))
        },
    )
    .unwrap();
    for _ in 0..3 {
        worker
            .command_sender()
            .send(ClipboardCommand::Capture)
            .unwrap();
    }
    worker
        .shutdown()
        .expect("shutdown drains captures without promoting a capture error");
    assert_eq!(invalidations.load(Ordering::SeqCst), 1);
    assert_eq!(entries.read().unwrap()[0].entry_id, "next-copy");
    assert_eq!(
        ClipboardDatabase::open(root.join("clipboard.db"))
            .unwrap()
            .load()
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn worker_panic_is_still_a_shutdown_failure() {
    use std::sync::mpsc;
    let (commands, receiver) = mpsc::sync_channel(1);
    let worker = super::ClipboardWorker {
        commands,
        thread: Some(std::thread::spawn(move || {
            let _ = receiver.recv();
            panic!("owner panic");
        })),
    };
    assert_eq!(worker.shutdown().unwrap_err(), "clipboard worker panicked");
}

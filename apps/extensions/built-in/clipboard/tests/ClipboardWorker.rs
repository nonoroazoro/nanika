use super::clear_entries;
use crate::{ClipboardDatabase, ClipboardEntry, ClipboardImageDimensions};
use crate::{ClipboardStore, ClipboardViewState};
use nanika_protocol::ClipboardContent;

#[test]
fn scoped_clear_preserves_other_payloads_and_publishes_committed_state() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-clear-payloads-{}",
        std::process::id()
    ));
    let payloads = root.join("payloads");
    std::fs::create_dir_all(&payloads).expect("payload root");
    let database = ClipboardDatabase::open(root.join("clipboard.db"), &payloads).expect("database");
    let paths = [
        payloads.join(format!("{}.png", "a".repeat(64))),
        payloads.join(format!("{}.png", "b".repeat(64)))
    ];
    for (index, path) in paths.iter().enumerate() {
        std::fs::write(path, b"payload").expect("payload");
        database
            .upsert(&ClipboardEntry {
                entry_id: path.file_stem().unwrap().to_str().unwrap().into(),
                image_dimensions: Some(ClipboardImageDimensions {
                    width: 1,
                    height: 1
                }),
                content: ClipboardContent::PngFile {
                    path: path.to_string_lossy().into_owned()
                },
                byte_size: 7,
                captured_at: index as u64
            })
            .expect("capture");
    }
    drop(database);
    let mut store = ClipboardStore::open(root.join("clipboard.db"), &payloads).unwrap();
    let mut payload_owner = crate::ClipboardPayloads::new(payloads.clone());
    clear_entries(&mut store, &mut payload_owner, &["a".repeat(64)], &|_| {})
        .expect("clear first image");
    assert!(!paths[0].exists());
    assert!(paths[1].exists());
    assert!(store.content(&"b".repeat(64)).is_ok());
    assert_eq!(
        store
            .present(ClipboardViewState::new(), None)
            .unwrap()
            .matching_ids
            .len(),
        1
    );

    let invalid_root = root.join("not-a-directory");
    std::fs::write(&invalid_root, b"not a directory").expect("failure fixture");
    let mut payload_owner = crate::ClipboardPayloads::new(invalid_root);
    clear_entries(&mut store, &mut payload_owner, &["b".repeat(64)], &|_| {})
        .expect_err("cleanup failure must be reported");
    assert!(
        store
            .present(ClipboardViewState::new(), None)
            .unwrap()
            .matching_ids
            .is_empty(),
        "the owner must reflect committed deletion despite cleanup failure"
    );
    drop(store);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn failed_capture_does_not_block_later_capture_or_poison_shutdown() {
    use crate::{ClipboardCommand, ClipboardConfig, ClipboardWorker};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering}
    };
    let root = std::env::temp_dir().join(format!("nanika-capture-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let invalidations = Arc::new(AtomicUsize::new(0));
    let changed = Arc::clone(&invalidations);
    let mut attempt = 0;
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(move |text| {
            assert_eq!(text.documents, Some(vec!["next-copy".to_owned()]));
            changed.fetch_add(1, Ordering::SeqCst);
        }),
        move |_| {
            attempt += 1;
            if attempt != 2 {
                return Err("clipboard text exceeds capture limit".into());
            }
            Ok(Some(ClipboardEntry {
                entry_id: "next-copy".into(),
                image_dimensions: None,
                content: ClipboardContent::Text {
                    value: "Next copy".into()
                },
                byte_size: 9,
                captured_at: 1
            }))
        }
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
    assert_eq!(
        ClipboardDatabase::open(root.join("clipboard.db"), &root.join("payloads"))
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
        _text_reader: crate::ClipboardTextReader::default(),
        commands,
        thread: Some(std::thread::spawn(move || {
            let _ = receiver.recv();
            panic!("owner panic");
        }))
    };
    assert_eq!(worker.shutdown().unwrap_err(), "clipboard worker panicked");
}
#[test]
fn capture_publishes_committed_state_when_payload_cleanup_fails() {
    use crate::{ClipboardCommand, ClipboardConfig, ClipboardWorker};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering}
    };
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-cleanup-failure-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let payloads = root.join("payloads");
    let changes = Arc::new(AtomicUsize::new(0));
    let notify = Arc::clone(&changes);
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        payloads.clone(),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(move |_| {
            notify.fetch_add(1, Ordering::SeqCst);
        }),
        |_| {
            Ok(Some(ClipboardEntry {
                entry_id: "committed".into(),
                image_dimensions: None,
                content: ClipboardContent::Text {
                    value: "saved".into()
                },
                byte_size: 5,
                captured_at: 1
            }))
        }
    )
    .unwrap();
    std::fs::write(&payloads, b"block cleanup after commit").unwrap();
    worker
        .command_sender()
        .send(ClipboardCommand::Capture)
        .unwrap();
    worker.shutdown().unwrap();
    assert_eq!(changes.load(Ordering::SeqCst), 1);
    assert_eq!(
        ClipboardDatabase::open(root.join("clipboard.db"), &root.join("payloads"))
            .unwrap()
            .load()
            .unwrap()[0]
            .entry_id,
        "committed"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retention_and_clear_notify_after_commit_even_when_payload_cleanup_fails() {
    use crate::{ClipboardConfig, ClipboardWorker};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering}
    };
    let root = std::env::temp_dir().join(format!(
        "nanika-retention-publication-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("clipboard.db");
    let database = ClipboardDatabase::open(&path, &root.join("payloads")).unwrap();
    for index in 0..3 {
        database
            .upsert(&ClipboardEntry {
                entry_id: format!("{index:064x}"),
                image_dimensions: Some(ClipboardImageDimensions {
                    width: 1,
                    height: 1
                }),
                content: ClipboardContent::PngFile {
                    path: root
                        .join("payloads")
                        .join(format!("{index:064x}.png"))
                        .to_string_lossy()
                        .into_owned()
                },
                byte_size: 1,
                captured_at: index
            })
            .unwrap();
    }
    drop(database);
    let changes = Arc::new(AtomicUsize::new(0));
    let changed = Arc::clone(&changes);
    let observer_path = path.clone();
    let payloads = root.join("payloads");
    let worker = ClipboardWorker::_spawn(
        path,
        payloads.clone(),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(move |_| {
            let observer = ClipboardDatabase::open(
                &observer_path,
                &observer_path.parent().unwrap().join("payloads")
            )
            .unwrap();
            let count = observer.matching_ids("", "all").unwrap().len();
            assert!(count <= 1, "notification follows the durable commit");
            changed.fetch_add(1, Ordering::SeqCst);
        }),
        |_| Ok(None)
    )
    .unwrap();
    for index in 0..3 {
        std::fs::create_dir_all(payloads.join(format!("{index:064x}.png"))).unwrap();
    }
    worker
        .apply_configuration(ClipboardConfig {
            syntax_highlighting: true,
            max_entries: Some(1),
            max_age_days: None
        })
        .unwrap();
    assert_eq!(
        worker
            .present(ClipboardViewState::new(), None)
            .unwrap()
            .matching_ids
            .len(),
        1
    );
    assert_eq!(changes.load(Ordering::SeqCst), 1);
    worker
        .clear_all()
        .expect_err("cleanup failure remains concrete");
    assert!(
        worker
            .present(ClipboardViewState::new(), None)
            .unwrap()
            .matching_ids
            .is_empty()
    );
    assert_eq!(changes.load(Ordering::SeqCst), 2);
    worker.shutdown().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn copy_leases_survive_capture_retention_until_the_final_reader_finishes() {
    use crate::{ClipboardCommand, ClipboardConfig, ClipboardWorker};
    use std::sync::Arc;
    let (root, image, entry) = seed_copy_image("capture");
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: Some(1),
            max_age_days: None
        },
        Arc::new(|_| {}),
        |_| {
            Ok(Some(ClipboardEntry {
                entry_id: "new-text".into(),
                image_dimensions: None,
                content: ClipboardContent::Text {
                    value: "Text".into()
                },
                byte_size: 4,
                captured_at: 2
            }))
        }
    )
    .unwrap();
    let first = worker.content(entry.entry_id.clone()).unwrap();
    let second = worker.content(entry.entry_id.clone()).unwrap();
    worker
        .command_sender()
        .send(ClipboardCommand::Capture)
        .unwrap();
    worker.present(ClipboardViewState::new(), None).unwrap();
    assert!(
        worker.content(entry.entry_id).is_err(),
        "retention commits while readers are active"
    );
    assert!(nanika_platform::read_png_resource(&image, &root.join("payloads")).is_ok());
    assert_eq!(first.content(), &entry.content);
    drop(first);
    worker.present(ClipboardViewState::new(), None).unwrap();
    assert!(image.exists(), "the second reader still owns the payload");
    drop(second);
    worker.present(ClipboardViewState::new(), None).unwrap();
    assert!(
        !image.exists(),
        "the final release reclaims the removed payload"
    );
    worker.shutdown().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recaptured_images_remain_owned_after_old_copy_leases_release() {
    use crate::{ClipboardCommand, ClipboardConfig, ClipboardWorker};
    use std::sync::Arc;
    let (root, image, entry) = seed_copy_image("recapture");
    let recaptured = entry.clone();
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(|_| {}),
        move |_| Ok(Some(recaptured.clone()))
    )
    .unwrap();
    let lease = worker.content(entry.entry_id.clone()).unwrap();
    worker.clear_all().unwrap();
    assert!(image.exists());
    worker
        .command_sender()
        .send(ClipboardCommand::Capture)
        .unwrap();
    drop(lease);
    worker.present(ClipboardViewState::new(), None).unwrap();
    assert!(
        image.exists(),
        "a new committed row owns the same immutable payload"
    );
    assert!(worker.content(entry.entry_id.clone()).is_ok());
    worker.clear_all().unwrap();
    assert!(!image.exists());
    worker.shutdown().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn abandoned_content_handoff_and_failed_consumers_release_their_payloads() {
    use crate::{ClipboardCommand, ClipboardConfig, ClipboardWorker};
    use std::sync::{Arc, mpsc};
    let (root, image, entry) = seed_copy_image("abandoned");
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(|_| {}),
        |_| Ok(None)
    )
    .unwrap();
    let (response, receiver) = mpsc::sync_channel(0);
    drop(receiver);
    worker
        .command_sender()
        .send(ClipboardCommand::Content {
            entry_id: entry.entry_id.clone(),
            response
        })
        .unwrap();
    let failure = (|| -> Result<(), String> {
        let _lease = worker.content(entry.entry_id.clone())?;
        worker.apply_configuration(ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: Some(1)
        })?;
        assert!(image.exists());
        assert!(worker.content(entry.entry_id).is_err());
        Err("host rejected copy".into())
    })();
    assert!(failure.is_err());
    worker.present(ClipboardViewState::new(), None).unwrap();
    assert!(
        !image.exists(),
        "neither abandoned handoff nor failed consumer leaks a reader"
    );
    worker.shutdown().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stored_image_paths_cannot_expand_payload_deletion_authority() {
    use crate::{ClipboardConfig, ClipboardWorker};
    use std::sync::Arc;
    let (root, image, mut entry) = seed_copy_image("external-path");
    let external = root.join(image.file_name().unwrap());
    std::fs::copy(&image, &external).unwrap();
    entry.image_dimensions = Some(ClipboardImageDimensions {
        width: 1,
        height: 1
    });
    entry.content = ClipboardContent::PngFile {
        path: external.to_string_lossy().into_owned()
    };
    ClipboardDatabase::open(root.join("clipboard.db"), &root.join("payloads"))
        .unwrap()
        .upsert(&entry)
        .unwrap();
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(|_| {}),
        |_| Ok(None)
    )
    .unwrap();
    assert!(worker.content(entry.entry_id.clone()).is_err());
    let error = worker.clear_all().unwrap_err();
    assert!(error.contains("Entries were removed; payload cleanup failed"));
    worker.shutdown().unwrap();
    assert!(
        external.exists(),
        "cleanup may only delete owned generated payloads"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn seed_copy_image(label: &str) -> (std::path::PathBuf, std::path::PathBuf, ClipboardEntry) {
    use sha2::{Digest, Sha256};
    let root = std::env::temp_dir().join(format!("nanika-copy-{label}-{}", std::process::id()));
    let payloads = root.join("payloads");
    std::fs::create_dir_all(&payloads).unwrap();
    let bytes = include_bytes!("../assets/icon.png");
    let mut digest = Sha256::new();
    digest.update(b"image\0");
    digest.update(bytes);
    let id = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let image = payloads.join(format!("{id}.png"));
    std::fs::write(&image, bytes).unwrap();
    let entry = ClipboardEntry {
        entry_id: id,
        image_dimensions: Some({
            let (width, height) = nanika_platform::png_resource_dimensions(bytes).unwrap();
            ClipboardImageDimensions { width, height }
        }),
        content: ClipboardContent::PngFile {
            path: image.to_string_lossy().into_owned()
        },
        byte_size: bytes.len() as u64,
        captured_at: 1
    };
    ClipboardDatabase::open(root.join("clipboard.db"), &root.join("payloads"))
        .unwrap()
        .upsert(&entry)
        .unwrap();
    (root, image, entry)
}

#[test]
fn presentation_configuration_does_not_trigger_retention() {
    use crate::{ClipboardConfig, ClipboardWorker};
    use std::sync::Arc;
    let root =
        std::env::temp_dir().join(format!("nanika-presentation-config-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let config = ClipboardConfig {
        syntax_highlighting: true,
        max_entries: Some(50),
        max_age_days: Some(1)
    };
    let (notices, received) = std::sync::mpsc::channel();
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        config.clone(),
        Arc::new(move |notice| {
            notices.send(notice).unwrap();
        }),
        |_| Ok(None)
    )
    .unwrap();
    // Model a row that has aged out while the owner was idle, after startup retention ran.
    let database =
        ClipboardDatabase::open(root.join("clipboard.db"), &root.join("payloads")).unwrap();
    database
        .upsert(&ClipboardEntry {
            entry_id: "text".into(),
            image_dimensions: None,
            content: ClipboardContent::Text {
                value: "# Heading".into()
            },
            byte_size: 9,
            captured_at: 0
        })
        .unwrap();
    worker
        .apply_configuration(ClipboardConfig {
            syntax_highlighting: false,
            ..config.clone()
        })
        .unwrap();
    assert!(received.recv().unwrap().is_empty());
    assert!(database.entry("text").is_ok());
    let presentation = worker.present(ClipboardViewState::new(), None).unwrap();
    let nanika_protocol::View::List { list } = presentation.view else {
        panic!("list")
    };
    assert!(matches!(
        list.detail.unwrap().content,
        nanika_protocol::DetailContent::Text {
            syntax_highlighting: false,
            ..
        }
    ));
    worker
        .apply_configuration(ClipboardConfig {
            max_age_days: Some(2),
            ..config
        })
        .unwrap();
    assert_eq!(received.recv().unwrap().documents.unwrap(), ["text"]);
    assert!(database.entry("text").is_err());
    worker.shutdown().unwrap();
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn settings_clear_removes_all_history_even_while_the_view_is_filtered() {
    use crate::{ClipboardConfig, ClipboardWorker};
    use std::sync::Arc;
    let (root, image, _) = seed_copy_image("settings-clear");
    let database =
        ClipboardDatabase::open(root.join("clipboard.db"), &root.join("payloads")).unwrap();
    database
        .upsert(&ClipboardEntry {
            entry_id: "text".into(),
            image_dimensions: None,
            content: ClipboardContent::Text {
                value: "Visible text".into()
            },
            byte_size: 12,
            captured_at: 2
        })
        .unwrap();
    drop(database);
    let worker = ClipboardWorker::_spawn(
        root.join("clipboard.db"),
        root.join("payloads"),
        ClipboardConfig {
            syntax_highlighting: true,
            max_entries: None,
            max_age_days: None
        },
        Arc::new(|_| {}),
        |_| Ok(None)
    )
    .unwrap();
    let mut filtered = ClipboardViewState::new();
    filtered.content_type = "text".into();
    assert_eq!(
        worker.present(filtered, None).unwrap().matching_ids.len(),
        1
    );
    worker.clear_all().unwrap();
    assert!(
        worker
            .present(ClipboardViewState::new(), None)
            .unwrap()
            .matching_ids
            .is_empty()
    );
    assert!(!image.exists());
    worker.shutdown().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

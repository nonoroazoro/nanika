use crate::{
    CLIPBOARD_PAGE_SIZE, ClipboardConfig, ClipboardDatabase, ClipboardEntry, ClipboardStore,
    ClipboardViewState,
};
use nanika_protocol::{ClipboardContent, DetailContent, View, ViewPageTarget};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn clear_scope_matches_type_and_query_before_pagination() {
    let mut entries = (0..105)
        .map(|i| _entry(i, "Shared needle"))
        .collect::<Vec<_>>();
    let mut files = _entry(106, "File");
    files.content = ClipboardContent::Files {
        paths: vec!["/example/needle.txt".into()],
    };
    entries.push(files);
    let mut image = _entry(107, "Image needle");
    image.entry_id = "a".repeat(64);
    image.content = ClipboardContent::PngFile {
        path: "image.png".into(),
    };
    entries.push(image);
    _with_store(&entries, |store, _| {
        for (kind, count) in [("text", 105), ("files", 1), ("images", 1), ("all", 107)] {
            let mut state = ClipboardViewState::new();
            state.query = "  NEEDLE  ".into();
            state.content_type = kind.into();
            let current = store.present(state, None).unwrap();
            assert_eq!(current.matching_ids.len(), count);
            let View::List { list } = current.view else {
                unreachable!()
            };
            assert_eq!(list.sections[0].items.len(), count.min(CLIPBOARD_PAGE_SIZE));
            let clear = list.sections[0].items[0]
                .actions
                .iter()
                .find(|a| a.id == crate::CLEAR_ACTION_ID)
                .unwrap();
            assert_eq!(clear.confirmation_title.as_deref(), Some("Clear now?"));
        }
        let mut state = ClipboardViewState::new();
        state.query = "missing".into();
        assert!(store.present(state, None).unwrap().matching_ids.is_empty());
    });
}

#[test]
fn every_history_page_is_bounded_and_all_retained_entries_remain_reachable() {
    let entries = (0..605).map(|i| _entry(i, "Entry")).collect::<Vec<_>>();
    _with_store(&entries, |store, _| {
        let mut current = store.present(ClipboardViewState::new(), None).unwrap();
        let mut seen = Vec::new();
        loop {
            let View::List { list } = &current.view else {
                unreachable!()
            };
            assert!(list.sections[0].items.len() <= CLIPBOARD_PAGE_SIZE);
            seen.extend(list.sections[0].items.iter().map(|i| i.id.clone()));
            let Some(cursor) = list.pagination.as_ref().and_then(|p| p.next_cursor.clone()) else {
                break;
            };
            let mut state = current.state.clone();
            crate::change_page(&mut state, &current.view, ViewPageTarget::List, &cursor).unwrap();
            current = store.present(state, Some(current.data_revision)).unwrap();
        }
        assert_eq!(
            seen,
            entries
                .iter()
                .rev()
                .map(|e| e.entry_id.clone())
                .collect::<Vec<_>>()
        );
        let mut state = current.state.clone();
        crate::change_page(&mut state, &current.view, ViewPageTarget::List, "590").unwrap();
        current = store.present(state, Some(current.data_revision)).unwrap();
        assert_eq!(current.state.selected_item_id.as_deref(), Some("14"));
        assert!(
            crate::change_page(
                &mut current.state.clone(),
                &current.view,
                ViewPageTarget::List,
                "99999"
            )
            .is_err()
        );
    });
}

#[test]
fn large_unicode_text_round_trips_through_pages_without_changing_the_copy_payload() {
    let original = "文字🙂\0\u{b}\n".repeat(30_000);
    _with_store(&[_entry(1, &original)], |store, _| {
        let mut current = store.present(ClipboardViewState::new(), None).unwrap();
        let mut displayed = String::new();
        loop {
            let View::List { list } = &current.view else {
                unreachable!()
            };
            let DetailContent::Text { value, pagination } = &list.detail.as_ref().unwrap().content
            else {
                unreachable!()
            };
            assert!(value.chars().count() <= 16_384);
            displayed.push_str(value);
            let Some(cursor) = pagination.as_ref().and_then(|p| p.next_cursor.clone()) else {
                break;
            };
            let mut state = current.state.clone();
            crate::change_page(&mut state, &current.view, ViewPageTarget::Detail, &cursor).unwrap();
            current = store.present(state, Some(current.data_revision)).unwrap();
        }
        assert_eq!(displayed, original);
        assert_eq!(
            store.content("1").unwrap(),
            ClipboardContent::Text {
                value: original.clone()
            }
        );
    });
}

#[test]
fn selection_reuses_query_ids_and_capture_invalidates_pages_without_losing_the_anchor() {
    let entries = (0..21)
        .map(|i| _entry(i, "Shared needle"))
        .collect::<Vec<_>>();
    _with_store(&entries, |store, _| {
        let mut state = ClipboardViewState::new();
        state.query = "needle".into();
        state.page_offset = 20;
        let current = store.present(state, None).unwrap();
        let same = store.present(current.state.clone(), None).unwrap();
        assert!(Arc::ptr_eq(&same.matching_ids, &current.matching_ids));
        store
            .capture(&_entry(100, "Shared needle"), 100, &_config())
            .unwrap();
        assert!(
            store
                .present(current.state.clone(), Some(current.data_revision))
                .is_err()
        );
        let updated = store.present(current.state, None).unwrap();
        assert!(!Arc::ptr_eq(&same.matching_ids, &updated.matching_ids));
        assert_eq!(updated.state.selected_item_id.as_deref(), Some("0"));
        store
            .apply_retention(
                100,
                &ClipboardConfig {
                    max_entries: Some(3),
                    max_age_days: None,
                },
            )
            .unwrap();
        let retained = store.present(updated.state, None).unwrap();
        assert_eq!(retained.state.page_offset, 0);
        assert_eq!(retained.state.selected_item_id.as_deref(), Some("100"));
    });
}

#[test]
fn control_characters_are_plain_query_data_and_failed_proposals_do_not_mutate_accepted_state() {
    _with_store(&[_entry(1, "before\u{1b}after")], |store, _| {
        let mut state = ClipboardViewState::new();
        state.query = "\u{1b}".into();
        let accepted = store.present(state, None).unwrap();
        assert_eq!(accepted.matching_ids.len(), 1);
        let mut proposed = accepted.state.clone();
        proposed.query = "x".repeat(4097);
        proposed.revision += 1;
        assert!(store.present(proposed, None).is_err());
        let retry = store.present(accepted.state.clone(), None).unwrap();
        assert_eq!(retry.state.revision, accepted.state.revision);
        assert_eq!(retry.state.query, "\u{1b}");
    });
}

#[test]
fn file_labels_escape_controls_while_original_paths_and_copy_payload_remain_intact() {
    let paths = vec![
        "/example/name\nwith\ttabs.txt".to_owned(),
        "/example/   ".into(),
    ];
    let mut entry = _entry(1, "name\nwith\ttabs.txt");
    entry.title = "name\nwith\ttabs.txt".into();
    entry.content = ClipboardContent::Files {
        paths: paths.clone(),
    };
    _with_store(&[entry], |store, _| {
        let current = store.present(ClipboardViewState::new(), None).unwrap();
        let View::List { list } = current.view else {
            unreachable!()
        };
        assert!(
            !list.sections[0].items[0]
                .title
                .chars()
                .any(char::is_control)
        );
        let DetailContent::Files { files } = list.detail.unwrap().content else {
            unreachable!()
        };
        assert_eq!(
            files.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
            paths
        );
        assert!(
            files
                .iter()
                .all(|f| !f.name.trim().is_empty() && !f.name.chars().any(char::is_control))
        );
        assert_eq!(
            store.content("1").unwrap(),
            ClipboardContent::Files {
                paths: paths.clone()
            }
        );
    });
}

#[test]
fn collection_preview_bounds_icon_work_and_waits_for_the_group_including_failures() {
    use nanika_protocol::{IconReference, ViewItemIcon};
    let paths = (0..5).map(|i| format!("/file-{i}.png")).collect::<Vec<_>>();
    let mut entry = _entry(1, "Group");
    entry.content = ClipboardContent::Files {
        paths: paths.clone(),
    };
    _with_store(&[entry], |store, _| {
        let reference = IconReference::new("a".repeat(64)).unwrap();
        for completed in 0..=3 {
            let mut current = store.present(ClipboardViewState::new(), None).unwrap();
            assert_eq!(
                current.icon_paths(),
                paths[..3]
                    .iter()
                    .map(std::path::PathBuf::from)
                    .collect::<Vec<_>>()
            );
            current.decorate_icons(&|path| {
                let index = paths
                    .iter()
                    .position(|p| std::path::Path::new(p) == path)
                    .unwrap();
                assert!(index < 3);
                (index < completed).then(|| (index != 2).then(|| reference.clone()))
            });
            current.view.validate().unwrap();
            let View::List { list } = current.view else {
                unreachable!()
            };
            assert_eq!(
                matches!(
                    list.sections[0].items[0].icon,
                    Some(ViewItemIcon::Native(_))
                ),
                completed > 0
            );
            let DetailContent::Files { files } = list.detail.unwrap().content else {
                unreachable!()
            };
            if completed < 3 {
                assert!(files.iter().all(|f| f.icon.is_none()));
            } else {
                assert_eq!(files[0].icon, Some(reference.clone()));
                assert_eq!(files[1].icon, Some(reference.clone()));
                assert!(files[2..].iter().all(|f| f.icon.is_none()));
            }
        }
    });
}

#[test]
fn native_file_icons_preserve_missing_files() {
    _with_store(&[], |store, root| {
        let path = root.join("file.txt");
        std::fs::write(&path, "example").unwrap();
        let mut entry = _entry(1, "File");
        entry.content = ClipboardContent::Files {
            paths: vec![
                path.to_string_lossy().into_owned(),
                root.join("missing.txt").to_string_lossy().into_owned(),
            ],
        };
        store.capture(&entry, 1, &_config()).unwrap();
        let mut icons = nanika_platform::FileIconCache::new(root.join("icons"));
        icons.get(&path).unwrap();
        let mut current = store.present(ClipboardViewState::new(), None).unwrap();
        current.decorate_icons(&|p| Some(icons.cached(p).ok().flatten()));
        current.view.validate().unwrap();
        let View::List { list } = current.view else {
            unreachable!()
        };
        let DetailContent::Files { files } = list.detail.unwrap().content else {
            unreachable!()
        };
        assert_eq!(files.len(), 2);
        assert!(files[0].icon.is_some());
        assert!(files[1].icon.is_none());
    });
}

#[test]
fn clear_uses_reviewed_ids_and_failed_transactions_preserve_query_authority() {
    _with_store(
        &[_entry(1, "needle"), _entry(2, "needle")],
        |store, root| {
            let reviewed = store.present(ClipboardViewState::new(), None).unwrap();
            let connection = rusqlite::Connection::open(root.join("clipboard.db")).unwrap();
            connection.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON clipboard_entries WHEN OLD.entry_id = '1' BEGIN SELECT RAISE(ABORT, 'test clear failure'); END;").unwrap();
            assert!(store.clear(&reviewed.matching_ids).is_err());
            let unchanged = store
                .present(reviewed.state.clone(), Some(reviewed.data_revision))
                .unwrap();
            assert!(Arc::ptr_eq(&reviewed.matching_ids, &unchanged.matching_ids));
            assert_eq!(unchanged.matching_ids.len(), 2);
            connection
                .execute_batch("DROP TRIGGER reject_delete")
                .unwrap();
            drop(connection);
            store.capture(&_entry(3, "needle"), 3, &_config()).unwrap();
            store.clear(&reviewed.matching_ids).unwrap();
            let remaining = store.present(reviewed.state, None).unwrap();
            assert_eq!(*remaining.matching_ids, ["3"]);
            assert_eq!(
                store.content("3").unwrap(),
                ClipboardContent::Text {
                    value: "needle".into()
                }
            );
        },
    );
}

#[test]
fn query_cache_preserves_unicode_matching_and_never_matches_across_file_boundaries() {
    let mut files = _entry(2, "Files");
    files.content = ClipboardContent::Files {
        paths: vec!["/one/end".into(), "/two/start".into()],
    };
    _with_store(&[_entry(1, "ÄBC"), files], |store, _| {
        let mut state = ClipboardViewState::new();
        state.query = "äbc".into();
        assert_eq!(
            *store.present(state.clone(), None).unwrap().matching_ids,
            ["1"]
        );
        state.query = "end/two".into();
        assert!(store.present(state, None).unwrap().matching_ids.is_empty());
    });
}

fn _entry(id: u64, value: &str) -> ClipboardEntry {
    ClipboardEntry {
        entry_id: id.to_string(),
        title: crate::text_title(value),
        content: ClipboardContent::Text {
            value: value.into(),
        },
        byte_size: value.len() as u64,
        captured_at: id,
    }
}
fn _config() -> ClipboardConfig {
    ClipboardConfig {
        max_entries: None,
        max_age_days: None,
    }
}
fn _with_store(
    entries: &[ClipboardEntry],
    test: impl FnOnce(&mut ClipboardStore, &std::path::Path),
) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "nanika-store-view-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let path = root.join("clipboard.db");
    let db = ClipboardDatabase::open(&path).unwrap();
    for entry in entries {
        db.upsert(entry).unwrap();
    }
    drop(db);
    let mut store = ClipboardStore::open(path).unwrap();
    test(&mut store, &root);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

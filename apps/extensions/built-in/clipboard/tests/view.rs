use crate::{
    CLIPBOARD_BATCH_SIZE, ClipboardConfig, ClipboardDatabase, ClipboardEntry, ClipboardStore,
    ClipboardViewState,
};
use nanika_protocol::{ClipboardContent, DetailContent, View};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn empty_history_and_unmatched_scopes_publish_distinct_copy() {
    _with_store(&[], |store, _| {
        for (query, content_type, filtered) in [
            ("", "all", false),
            ("   ", "all", false),
            ("missing", "all", true),
            ("", "text", true),
            ("", "files", true),
            ("", "images", true),
            ("missing", "images", true),
        ] {
            let mut state = ClipboardViewState::new();
            state.query = query.into();
            state.content_type = content_type.into();
            let current = store.present(state, None).unwrap();
            current.view.validate().unwrap();
            let View::List { list } = current.view else {
                unreachable!()
            };
            assert_eq!(list.total(), 0);
            assert!(list.sections.iter().all(|section| section.items.is_empty()));
            assert_eq!(list.search_placeholder, "Search clipboard history");
            assert_eq!(
                list.empty_title,
                if filtered {
                    "No matching entries"
                } else {
                    "No clipboard history yet"
                }
            );
            assert_eq!(
                list.empty_description,
                if filtered {
                    "Try a different search or filter."
                } else {
                    "Copied content will appear here."
                }
            );
        }
    });
}

#[test]
fn clear_scope_matches_full_type_and_query() {
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
            assert_eq!(
                list.sections[0].items.len(),
                count.min(CLIPBOARD_BATCH_SIZE)
            );
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
fn bounded_windows_reach_all_records_and_keep_offscreen_selection() {
    let entries = (0..605).map(|i| _entry(i, "Entry")).collect::<Vec<_>>();
    _with_store(&entries, |store, _| {
        let mut current = store.present(ClipboardViewState::new(), None).unwrap();
        let mut seen = Vec::new();
        loop {
            let View::List { list } = &current.view else {
                unreachable!()
            };
            assert!(list.sections[0].items.len() <= CLIPBOARD_BATCH_SIZE);
            assert_eq!(list.selection.as_ref().unwrap().item.id, "604");
            seen.extend(list.sections[0].items.iter().map(|item| item.id.clone()));
            let offset = list.sections[0].offset + list.sections[0].items.len();
            if offset == list.total() {
                break;
            }
            let mut state = current.state.clone();
            crate::read_range(
                &mut state,
                &current.view,
                &list.collection_id,
                offset,
                std::num::NonZeroU32::new(10).unwrap(),
            )
            .unwrap();
            current = store
                .present(state, Some(current.collection_revision))
                .unwrap();
        }
        assert_eq!(
            seen,
            entries
                .iter()
                .rev()
                .map(|entry| entry.entry_id.clone())
                .collect::<Vec<_>>()
        );
        let View::List { list } = &current.view else {
            unreachable!()
        };
        let mut state = current.state.clone();
        crate::read_range(
            &mut state,
            &current.view,
            &list.collection_id,
            0,
            std::num::NonZeroU32::new(10).unwrap(),
        )
        .unwrap();
        let first = store
            .present(state, Some(current.collection_revision))
            .unwrap();
        assert_eq!(first.state.offset, 0);
    });
}

#[test]
fn unicode_text_chunks_are_bounded_independent_and_preserve_the_full_copy_payload() {
    let original = "文字🙂\0\u{b}\n".repeat(30_000);
    _with_store(&[_entry(1, &original)], |store, _| {
        let mut current = store.present(ClipboardViewState::new(), None).unwrap();
        let mut displayed = String::new();
        loop {
            let View::List { list } = &current.view else {
                unreachable!()
            };
            let DetailContent::Text {
                value,
                text_id,
                chunk_index,
                total_chunks,
            } = &list.detail.as_ref().unwrap().content
            else {
                unreachable!()
            };
            assert!(value.chars().count() <= nanika_protocol::DETAIL_TEXT_BATCH_CHARS);
            displayed.push_str(value);
            if chunk_index + 1 == *total_chunks {
                break;
            }
            let mut state = current.state.clone();
            crate::read_text_chunk(&mut state, &current.view, text_id, chunk_index + 1).unwrap();
            current = store.present(state, None).unwrap();
        }
        assert_eq!(displayed, original);
        assert_eq!(
            store.content("1").unwrap(),
            ClipboardContent::Text { value: original }
        );
        let mut state = current.state.clone();
        crate::read_text_chunk(&mut state, &current.view, "1", 0).unwrap();
        assert!(crate::read_text_chunk(&mut state, &current.view, "other", 0).is_err());
        let first = store.present(state, None).unwrap();
        assert_eq!(first.state.text_chunk, 0);
    });
}

#[test]
fn search_reads_unloaded_entries_and_the_full_text_payload() {
    let mut entries = (0..75)
        .map(|i| _entry(i, "Ordinary record"))
        .collect::<Vec<_>>();
    let text = format!("{}hidden-tail-needle", "prefix ".repeat(20_000));
    entries[0] = _entry(0, &text);
    _with_store(&entries, |store, _| {
        let initial = store.present(ClipboardViewState::new(), None).unwrap();
        let View::List { list } = &initial.view else {
            unreachable!()
        };
        assert!(!list.sections[0].items.iter().any(|item| item.id == "0"));
        let mut state = ClipboardViewState::new();
        state.query = "hidden-tail-needle".into();
        let result = store.present(state, None).unwrap();
        assert_eq!(&*result.matching_ids, &["0".to_owned()]);
        let View::List { list } = &result.view else {
            unreachable!()
        };
        assert_eq!(list.sections[0].items[0].id, "0");
        assert_eq!(list.total(), 1);
        assert_eq!(
            store.content("0").unwrap(),
            ClipboardContent::Text {
                value: text.clone()
            }
        );
    });
}

#[test]
fn range_reads_reject_foreign_collections_and_invalid_ranges_without_mutating_state() {
    let entries = (0..35).map(|i| _entry(i, "Entry")).collect::<Vec<_>>();
    _with_store(&entries, |store, _| {
        let mut state = ClipboardViewState::new();
        state.selected_item_id = Some("32".into());
        let current = store.present(state, None).unwrap();
        let View::List { list } = &current.view else {
            unreachable!()
        };
        for (id, offset, count) in [
            ("other", 0, 10),
            (list.collection_id.as_str(), 35, 10),
            (list.collection_id.as_str(), 0, 501),
        ] {
            let mut proposed = current.state.clone();
            assert!(
                crate::read_range(
                    &mut proposed,
                    &current.view,
                    id,
                    offset,
                    std::num::NonZeroU32::new(count).unwrap()
                )
                .is_err()
            );
            assert_eq!(proposed, current.state);
        }
    });
}

#[test]
fn selection_reuses_query_ids_and_capture_invalidates_continuations_without_losing_the_anchor() {
    let entries = (0..21)
        .map(|i| _entry(i, "Shared needle"))
        .collect::<Vec<_>>();
    _with_store(&entries, |store, _| {
        let mut state = ClipboardViewState::new();
        state.query = "needle".into();
        state.count = 21;
        state.selected_item_id = Some("0".into());
        let current = store.present(state, None).unwrap();
        let same = store.present(current.state.clone(), None).unwrap();
        assert!(Arc::ptr_eq(&same.matching_ids, &current.matching_ids));
        store
            .capture(&_entry(100, "Shared needle"), 100, &_config())
            .unwrap();
        assert!(
            store
                .present(current.state.clone(), Some(current.collection_revision))
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
        assert_eq!(retained.state.count, 21);
        assert_eq!(retained.state.selected_item_id.as_deref(), Some("100"));
        assert_eq!(retained.state.anchor_id.as_deref(), Some("100"));
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
                .present(reviewed.state.clone(), Some(reviewed.collection_revision))
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

#[test]
fn searches_publish_the_requested_window_without_intermediate_small_batches() {
    let mut entries = (0..105)
        .map(|i| _entry(i, "Image record"))
        .collect::<Vec<_>>();
    entries[0] = _entry(0, "Unique needle outside the initial prefix");
    _with_store(&entries, |store, _| {
        let mut state = ClipboardViewState::new();
        for (query, demand, expected) in [
            ("i", 27, 27),
            ("im", 27, 27),
            ("ima", 43, 43),
            ("image", 43, 43),
            ("missing", 43, 0),
            ("unique needle", 43, 1),
            ("", 27, 27),
        ] {
            state.query = query.into();
            state.reset_results(std::num::NonZeroU32::new(demand).unwrap());
            let current = store.present(state, None).unwrap();
            let View::List { list } = &current.view else {
                unreachable!()
            };
            let count: usize = list
                .sections
                .iter()
                .map(|section| section.items.len())
                .sum();
            assert_eq!(count, expected, "query {query}");
            assert_eq!(list.total(), current.matching_ids.len());
            if query == "unique needle" {
                assert_eq!(
                    list.selection
                        .as_ref()
                        .map(|selection| selection.item.id.as_str()),
                    Some("0")
                );
            }
            state = current.state;
        }
        state.content_type = "images".into();
        state.reset_results(std::num::NonZeroU32::new(27).unwrap());
        assert!(
            store
                .present(state.clone(), None)
                .unwrap()
                .matching_ids
                .is_empty()
        );
        state.content_type = "all".into();
        state.reset_results(std::num::NonZeroU32::new(27).unwrap());
        let current = store.present(state, None).unwrap();
        let View::List { list } = &current.view else {
            unreachable!()
        };
        assert_eq!(list.sections[0].items.len(), 27);
        let mut next = current.state.clone();
        crate::read_range(
            &mut next,
            &current.view,
            &list.collection_id,
            27,
            std::num::NonZeroU32::new(27).unwrap(),
        )
        .unwrap();
        let grown = store
            .present(next, Some(current.collection_revision))
            .unwrap();
        let View::List { list } = &grown.view else {
            unreachable!()
        };
        assert_eq!(list.sections[0].items.len(), 27);
        assert_eq!(list.sections[0].offset, 27);
    });
}

#[test]
fn selection_resolves_against_full_reviewed_order_after_a_window_replacement() {
    let entries = (0..100).map(|i| _entry(i, "Entry")).collect::<Vec<_>>();
    _with_store(&entries, |store, _| {
        let current = store.present(ClipboardViewState::new(), None).unwrap();
        let View::List { list } = &current.view else {
            unreachable!()
        };
        let collection_id = list.collection_id.clone();
        let mut shifted = current.state.clone();
        crate::read_range(
            &mut shifted,
            &current.view,
            &collection_id,
            50,
            std::num::NonZeroU32::new(10).unwrap(),
        )
        .unwrap();
        let shifted = store
            .present(shifted, Some(current.collection_revision))
            .unwrap();
        let selected = shifted.select_index(&collection_id, 1).unwrap();
        let updated = store
            .present(selected, Some(shifted.collection_revision))
            .unwrap();
        let View::List { list } = &updated.view else {
            unreachable!()
        };
        assert_eq!(list.selection.as_ref().unwrap().index, 1);
        assert_eq!(list.selection.as_ref().unwrap().item.id, "98");
        assert_eq!(list.sections[0].offset, 50);
        assert!(updated.select_index("other", 1).is_err());
        assert!(updated.select_index(&collection_id, 100).is_err());
        store
            .capture(&_entry(101, "Entry"), 101, &_config())
            .unwrap();
        assert!(
            store
                .present(
                    updated.select_index(&collection_id, 1).unwrap(),
                    Some(updated.collection_revision)
                )
                .is_err()
        );
    });
}

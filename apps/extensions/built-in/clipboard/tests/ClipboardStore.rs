use crate::{
    ClipboardConfig, ClipboardDatabase, ClipboardEntry, ClipboardStore, ClipboardViewState,
};
use nanika_protocol::ClipboardContent;
use std::sync::Arc;

#[test]
fn incremental_queries_match_fresh_database_scans_after_committed_mutations() {
    let root =
        std::env::temp_dir().join(format!("nanika-incremental-query-{}", std::process::id()));
    let path = root.join("clipboard.db");
    let config = ClipboardConfig {
        max_entries: None,
        max_age_days: None,
    };
    let mut store = ClipboardStore::open(&path).unwrap();
    let database = ClipboardDatabase::open(&path).unwrap();
    for filter in ["all", "text", "files", "images"] {
        for query in ["", "needle", "ä"] {
            let mut state = ClipboardViewState::new();
            state.content_type = filter.into();
            state.query = query.into();
            let mut current = store.present(state, None).unwrap();
            for (id, captured_at, content) in [
                (
                    "a",
                    10,
                    ClipboardContent::Text {
                        value: "needle Ä".into(),
                    },
                ),
                (
                    "b",
                    10,
                    ClipboardContent::Files {
                        paths: vec!["C:/needle.txt".into()],
                    },
                ),
                (
                    "c",
                    8,
                    ClipboardContent::Text {
                        value: "other".into(),
                    },
                ),
                (
                    "a",
                    5,
                    ClipboardContent::Text {
                        value: "other".into(),
                    },
                ),
                (
                    "a",
                    20,
                    ClipboardContent::Text {
                        value: "NEEDLE".into(),
                    },
                ),
                (
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    u64::MAX,
                    ClipboardContent::PngFile {
                        path: format!("{}.png", "a".repeat(64)),
                    },
                ),
                (
                    "b",
                    20,
                    ClipboardContent::Text {
                        value: "needle ä".into(),
                    },
                ),
            ] {
                let reviewed = Arc::clone(&current.matching_ids);
                let reviewed_values = (*reviewed).clone();
                store
                    .capture(
                        &ClipboardEntry {
                            entry_id: id.into(),
                            title: "Saved".into(),
                            content,
                            byte_size: 1,
                            captured_at,
                        },
                        captured_at,
                        &config,
                    )
                    .unwrap();
                let next = store.present(current.state.clone(), None).unwrap();
                assert_eq!(
                    store
                        .present(current.state.clone(), Some(current.collection_revision))
                        .is_err(),
                    current.collection_revision != next.collection_revision
                );
                current = next;
                assert_eq!(
                    *current.matching_ids,
                    database.matching_ids(query, filter).unwrap()
                );
                assert_eq!(
                    *reviewed, reviewed_values,
                    "published clear scopes must stay immutable"
                );
            }
            store.clear(&["a".into(), "missing".into()]).unwrap();
            current = store.present(current.state, None).unwrap();
            assert_eq!(
                *current.matching_ids,
                database.matching_ids(query, filter).unwrap()
            );
            store
                .apply_retention(
                    100,
                    &ClipboardConfig {
                        max_entries: Some(1),
                        max_age_days: None,
                    },
                )
                .unwrap();
            current = store.present(current.state, None).unwrap();
            assert_eq!(
                *current.matching_ids,
                database.matching_ids(query, filter).unwrap()
            );
            // An admitted capture can itself be removed by the same retention transaction.
            store
                .capture(
                    &ClipboardEntry {
                        entry_id: "expired".into(),
                        title: "needle".into(),
                        content: ClipboardContent::Text {
                            value: "needle".into(),
                        },
                        byte_size: 6,
                        captured_at: 0,
                    },
                    100,
                    &ClipboardConfig {
                        max_entries: Some(1),
                        max_age_days: None,
                    },
                )
                .unwrap();
            current = store.present(current.state, None).unwrap();
            assert_eq!(
                *current.matching_ids,
                database.matching_ids(query, filter).unwrap()
            );
        }
    }
    drop(database);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unmatched_capture_and_removal_preserve_query_identity() {
    let mut store = ClipboardStore::open(":memory:").unwrap();
    let mut state = ClipboardViewState::new();
    state.query = "needle".into();
    let current = store.present(state, None).unwrap();
    store
        .capture(
            &ClipboardEntry {
                entry_id: "unmatched".into(),
                title: "other".into(),
                content: ClipboardContent::Text {
                    value: "other".into(),
                },
                byte_size: 5,
                captured_at: 1,
            },
            1,
            &ClipboardConfig {
                max_entries: None,
                max_age_days: None,
            },
        )
        .unwrap();
    let after_capture = store
        .present(current.state.clone(), Some(current.collection_revision))
        .unwrap();
    assert_eq!(
        current.collection_revision,
        after_capture.collection_revision
    );
    assert!(Arc::ptr_eq(
        &current.matching_ids,
        &after_capture.matching_ids
    ));
    store.clear(&["unmatched".into()]).unwrap();
    let after_clear = store
        .present(current.state, Some(current.collection_revision))
        .unwrap();
    assert_eq!(current.collection_revision, after_clear.collection_revision);
    assert!(Arc::ptr_eq(
        &current.matching_ids,
        &after_clear.matching_ids
    ));
}

#[test]
fn closing_a_view_releases_all_transient_query_and_preview_caches() {
    let mut store = crate::ClipboardStore::open(":memory:").unwrap();
    store
        .capture(
            &ClipboardEntry {
                entry_id: "entry".into(),
                title: "Entry".into(),
                content: ClipboardContent::Text {
                    value: "Document".into(),
                },
                byte_size: 8,
                captured_at: 1,
            },
            1,
            &ClipboardConfig {
                max_entries: None,
                max_age_days: None,
            },
        )
        .unwrap();
    store
        .present(crate::ClipboardViewState::new(), None)
        .unwrap();
    assert!(store._preview.is_some());
    assert!(store._query.is_some());
    assert!(store._window.is_some());
    store.close_view();
    assert!(store._query.is_none());
    assert!(store._window.is_none());
    assert!(store._preview.is_none());
    assert_eq!(
        store.content("entry").unwrap(),
        ClipboardContent::Text {
            value: "Document".into()
        }
    );
}

use crate::{
    CLIPBOARD_PAGE_SIZE, ClipboardEntry, ClipboardViewState, clipboard_view, matching_entries,
};
use nanika_protocol::{ClipboardContent, View};

#[test]
fn clear_scope_matches_type_and_query_before_pagination() {
    let mut entries = (0..105)
        .map(|index| ClipboardEntry {
            entry_id: format!("text.{index}"),
            content_hash: format!("text.{index}"),
            title: format!("Note {index}"),
            content: ClipboardContent::Text {
                value: "Shared needle".to_owned(),
            },
            byte_size: 13,
            captured_at: index,
        })
        .collect::<Vec<_>>();
    entries.push(ClipboardEntry {
        entry_id: "files".to_owned(),
        content_hash: "files".to_owned(),
        title: "File".to_owned(),
        content: ClipboardContent::Files {
            paths: vec!["/example/needle.txt".to_owned()],
        },
        byte_size: 1,
        captured_at: 106,
    });
    entries.push(ClipboardEntry {
        entry_id: "image".to_owned(),
        content_hash: "image".to_owned(),
        title: "Image needle".to_owned(),
        content: ClipboardContent::PngFile {
            path: "image.png".to_owned(),
        },
        byte_size: 1,
        captured_at: 107,
    });
    let mut state = ClipboardViewState::new();
    state.query = "  NEEDLE  ".to_owned();
    for (kind, count) in [("text", 105), ("files", 1), ("images", 1), ("all", 107)] {
        state.content_type = kind.to_owned();
        let matches = matching_entries(&state, &entries);
        assert_eq!(matches.len(), count);
        let View::List { list } = clipboard_view(&mut state, &entries) else {
            panic!("list expected")
        };
        assert_eq!(
            list.sections
                .iter()
                .flat_map(|section| &section.items)
                .map(|item| &item.id)
                .collect::<Vec<_>>(),
            matches
                .iter()
                .take(CLIPBOARD_PAGE_SIZE)
                .map(|entry| &entry.entry_id)
                .collect::<Vec<_>>()
        );
    }
    state.query = "not present".to_owned();
    assert!(matching_entries(&state, &entries).is_empty());
    state.query = "Note 104".to_owned();
    state.content_type = "text".to_owned();
    let View::List { list } = clipboard_view(&mut state, &entries) else {
        panic!("list expected")
    };
    assert_eq!(list.sections[0].items[0].id, "text.104");
    state.query.clear();
    state.content_type = "all".to_owned();
    assert_eq!(matching_entries(&state, &entries).len(), entries.len());
}

#[test]
fn file_views_include_display_paths_and_native_icons_without_removing_missing_files() {
    use crate::render_clipboard_view;
    use nanika_protocol::{DetailContent, ViewItemIcon};
    use std::sync::RwLock;
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-file-view-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("test root");
    let path = root.join("file.txt");
    let second_path = root.join("second.txt");
    std::fs::write(&path, "example").expect("file fixture");
    std::fs::write(&second_path, "example").expect("second file fixture");
    let entries = RwLock::new(
        [path.clone(), root.join("missing.txt")]
            .iter()
            .enumerate()
            .map(|(index, path)| ClipboardEntry {
                entry_id: index.to_string(),
                content_hash: index.to_string(),
                title: "File".to_owned(),
                content: ClipboardContent::Files {
                    paths: if index == 0 {
                        vec![
                            path.to_string_lossy().into_owned(),
                            second_path.to_string_lossy().into_owned(),
                        ]
                    } else {
                        vec![path.to_string_lossy().into_owned()]
                    },
                },
                byte_size: 1,
                captured_at: 1,
            })
            .collect(),
    );
    let mut state = ClipboardViewState::new();
    let mut icons = nanika_platform::FileIconCache::new(root.join("icons"));
    let initial = render_clipboard_view(&mut state, &entries, &|path| {
        icons.cached(path).ok().flatten().map(Some)
    });
    let View::List { list } = initial else {
        panic!("list expected")
    };
    assert_eq!(list.sections[0].items[0].icon, Some(ViewItemIcon::Files));
    let DetailContent::Files { files } = &list.detail.expect("initial detail").content else {
        panic!("files expected")
    };
    assert!(files.iter().all(|file| file.icon.is_none()));

    icons.get(&path).expect("native file icon");
    let view = render_clipboard_view(&mut state, &entries, &|path| {
        Some(icons.cached(path).ok().flatten())
    });
    view.validate().expect("valid native file view");
    let View::List { list } = view else {
        panic!("list expected")
    };
    assert_eq!(list.sections[0].items.len(), 2);
    let Some(ViewItemIcon::Native(reference)) = &list.sections[0].items[0].icon else {
        panic!("native icon expected")
    };
    assert_eq!(list.sections[0].items[1].icon, Some(ViewItemIcon::Files));
    let DetailContent::Files { files } = &list.detail.expect("selected detail").content else {
        panic!("files expected")
    };
    assert_eq!(files[0].name, "file.txt");
    assert_eq!(files[0].path, path.to_str().expect("fixture path"));
    assert_eq!(files[0].icon.as_ref(), Some(reference));
    assert_eq!(files[1].path, second_path.to_str().expect("second path"));
    assert_eq!(
        files[1].icon, None,
        "failed files retain the semantic fallback without blocking the group"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn multi_file_detail_bounds_icon_lookups_to_the_collection_preview() {
    use crate::{FILE_COLLECTION_PREVIEW_LIMIT, render_clipboard_view};
    use std::path::PathBuf;
    use std::sync::{Mutex, RwLock};

    let paths = (0..FILE_COLLECTION_PREVIEW_LIMIT + 2)
        .map(|index| format!("/example/file-{index}.txt"))
        .collect::<Vec<_>>();
    let entries = RwLock::new(vec![ClipboardEntry {
        entry_id: "collection".to_owned(),
        content_hash: "collection".to_owned(),
        title: "Collection".to_owned(),
        content: ClipboardContent::Files {
            paths: paths.clone(),
        },
        byte_size: 1,
        captured_at: 1,
    }]);
    let requested = Mutex::new(Vec::new());

    let mut state = ClipboardViewState::new();
    let view = render_clipboard_view(&mut state, &entries, &|path| {
        requested
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(path.to_owned());
        None
    });

    view.validate().expect("valid collection view");
    assert_eq!(
        *requested.lock().unwrap_or_else(|error| error.into_inner()),
        paths
            .iter()
            .take(FILE_COLLECTION_PREVIEW_LIMIT)
            .map(PathBuf::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn collection_preview_waits_for_the_group_and_settles_failed_members() {
    use nanika_protocol::{DetailContent, IconReference, ViewItemIcon};
    use std::sync::RwLock;

    let paths = (0..4).map(|i| format!("/file-{i}.png")).collect::<Vec<_>>();
    let entries = RwLock::new(vec![ClipboardEntry {
        entry_id: "group".to_owned(),
        content_hash: "group".to_owned(),
        title: "Group".to_owned(),
        content: ClipboardContent::Files {
            paths: paths.clone(),
        },
        byte_size: 1,
        captured_at: 1,
    }]);
    let reference = IconReference::new("a".repeat(64)).unwrap();
    let mut state = ClipboardViewState::new();
    for completed in 0..=3 {
        let view = crate::render_clipboard_view(&mut state, &entries, &|path| {
            let index = paths
                .iter()
                .position(|candidate| std::path::Path::new(candidate) == path)
                .unwrap();
            (index < completed).then(|| (index != 2).then(|| reference.clone()))
        });
        let View::List { list } = view else {
            panic!("list expected")
        };
        assert_eq!(
            matches!(
                list.sections[0].items[0].icon,
                Some(ViewItemIcon::Native(_))
            ),
            completed > 0
        );
        let DetailContent::Files { files } = list.detail.unwrap().content else {
            panic!("files expected")
        };
        if completed < 3 {
            assert!(files.iter().all(|file| file.icon.is_none()));
        } else {
            assert_eq!(files[0].icon, Some(reference.clone()));
            assert_eq!(files[1].icon, Some(reference.clone()));
            assert!(files[2].icon.is_none(), "failed member retains fallback");
            assert!(files[3].icon.is_none(), "no work beyond the preview bound");
        }
    }
}

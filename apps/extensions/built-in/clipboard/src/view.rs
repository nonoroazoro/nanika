use std::path::Path;

use nanika_protocol::{
    Action, ActionStyle, ClipboardContent, DetailContent, DetailView, ImageSource, ListItem,
    ListLayout, ListSection, ListView, View, ViewFile, ViewFilter, ViewFilterOption, ViewItemIcon,
    ViewMetadata,
};

use crate::{CLEAR_ACTION_ID, COPY_ACTION_ID, ClipboardEntry, ClipboardViewState};

pub const FILE_COLLECTION_PREVIEW_LIMIT: usize = 3;

pub fn clipboard_view(state: &mut ClipboardViewState, entries: &[ClipboardEntry]) -> View {
    let matching = matching_entries(state, entries);
    let visible = matching
        .iter()
        .copied()
        .take(state.visible_limit)
        .collect::<Vec<_>>();
    if state
        .selected_item_id
        .as_ref()
        .is_none_or(|selected| !visible.iter().any(|entry| entry.entry_id == *selected))
    {
        state.selected_item_id = visible.first().map(|entry| entry.entry_id.clone());
    }
    let selected = state
        .selected_item_id
        .as_deref()
        .and_then(|selected| visible.iter().find(|entry| entry.entry_id == selected))
        .copied();
    let sections = (!visible.is_empty())
        .then(|| ListSection {
            id: "all".to_owned(),
            title: None,
            items: visible.iter().map(|entry| list_item(entry)).collect(),
        })
        .into_iter()
        .collect();
    View::List {
        list: Box::new(ListView {
            title: "Clipboard History".to_owned(),
            search_placeholder: "Search for entries".to_owned(),
            search_text: state.query.clone(),
            layout: ListLayout::Split,
            sections,
            selected_item_id: state.selected_item_id.clone(),
            detail: selected.map(detail_view),
            filter: Some(ViewFilter {
                id: "contentType".to_owned(),
                selected_value: state.content_type.clone(),
                options: vec![
                    filter_option("all", "All Types"),
                    filter_option("text", "Text"),
                    filter_option("files", "Files"),
                    filter_option("images", "Images"),
                ],
            }),
            next_cursor: (matching.len() > visible.len()).then(|| visible.len().to_string()),
        }),
    }
}

/// Listing and clearing share the same scope, before pagination is applied.
pub fn matching_entries<'a>(
    state: &ClipboardViewState,
    entries: &'a [ClipboardEntry],
) -> Vec<&'a ClipboardEntry> {
    entries
        .iter()
        .filter(|entry| matches_content_type(entry, &state.content_type))
        .filter(|entry| matches_query(entry, &state.query))
        .collect()
}

fn list_item(entry: &ClipboardEntry) -> ListItem {
    ListItem {
        id: entry.entry_id.clone(),
        title: entry.title.clone(),
        subtitle: None,
        icon: Some(match &entry.content {
            ClipboardContent::Text { .. } => ViewItemIcon::Text,
            ClipboardContent::Files { .. } => ViewItemIcon::Files,
            ClipboardContent::PngFile { .. } => ViewItemIcon::Image,
        }),
        actions: vec![clear_action(), copy_action()],
    }
}

fn detail_view(entry: &ClipboardEntry) -> DetailView {
    DetailView {
        title: None,
        content: match &entry.content {
            ClipboardContent::Text { value } => DetailContent::Text {
                value: value.clone(),
            },
            ClipboardContent::Files { paths } => DetailContent::Files {
                files: paths
                    .iter()
                    .map(|path| ViewFile {
                        path: path.clone(),
                        icon: None,
                        name: Path::new(path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(path)
                            .to_owned(),
                    })
                    .collect(),
            },
            ClipboardContent::PngFile { .. }
                if nanika_protocol::is_valid_content_hash(&entry.entry_id) =>
            {
                DetailContent::Image {
                    source: ImageSource::Resource {
                        path: format!("{}.png", entry.entry_id),
                    },
                    alternative_text: entry.title.clone(),
                }
            }
            ClipboardContent::PngFile { .. } => DetailContent::Text {
                value: "Image preview unavailable".to_owned(),
            },
        },
        metadata: vec![
            ViewMetadata {
                title: "Content type".to_owned(),
                value: content_type(entry).to_owned(),
            },
            ViewMetadata {
                title: "Size".to_owned(),
                value: format_bytes(entry.byte_size),
            },
        ],
        actions: Vec::new(),
    }
}

fn copy_action() -> Action {
    Action {
        id: COPY_ACTION_ID.to_owned(),
        title: "Copy to Clipboard".to_owned(),
        confirmation_title: None,
        allow_default_execution: true,
        style: ActionStyle::Primary,
        enabled: true,
        group: None,
    }
}

fn clear_action() -> Action {
    Action {
        id: CLEAR_ACTION_ID.to_owned(),
        title: "Clear history".to_owned(),
        confirmation_title: Some("Clear now?".to_owned()),
        allow_default_execution: false,
        style: ActionStyle::Destructive,
        enabled: true,
        group: None,
    }
}

fn filter_option(value: &str, title: &str) -> ViewFilterOption {
    ViewFilterOption {
        value: value.to_owned(),
        title: title.to_owned(),
    }
}

fn matches_query(entry: &ClipboardEntry, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || contains_query(&entry.title, &query)
        || match &entry.content {
            ClipboardContent::Text { value } => contains_query(value, &query),
            ClipboardContent::Files { paths } => {
                paths.iter().any(|path| contains_query(path, &query))
            }
            ClipboardContent::PngFile { .. } => false,
        }
}

fn contains_query(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

fn matches_content_type(entry: &ClipboardEntry, selected: &str) -> bool {
    selected == "all"
        || matches!(
            (&entry.content, selected),
            (ClipboardContent::Text { .. }, "text")
                | (ClipboardContent::Files { .. }, "files")
                | (ClipboardContent::PngFile { .. }, "images")
        )
}

fn content_type(entry: &ClipboardEntry) -> &'static str {
    match entry.content {
        ClipboardContent::Text { .. } => "Text",
        ClipboardContent::Files { .. } => "Files",
        ClipboardContent::PngFile { .. } => "Image",
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1_024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KiB", bytes as f64 / 1_024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    }
}

/// Native icon work happens after releasing the clipboard owner's shared snapshot lock.
pub fn render_clipboard_view(
    state: &mut ClipboardViewState,
    entries: &std::sync::RwLock<Vec<ClipboardEntry>>,
    icon_for_path: &impl Fn(&Path) -> Option<Option<nanika_protocol::IconReference>>,
) -> View {
    let (mut view, paths) = {
        let entries = entries.read().unwrap_or_else(|error| error.into_inner());
        let view = clipboard_view(state, &entries);
        let View::List { list } = &view else {
            unreachable!()
        };
        let visible_ids = list
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .map(|item| item.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let paths = entries
            .iter()
            .filter_map(|entry| {
                if !visible_ids.contains(entry.entry_id.as_str()) {
                    return None;
                }
                let ClipboardContent::Files { paths } = &entry.content else {
                    return None;
                };
                Some((entry.entry_id.clone(), paths.clone()))
            })
            .collect::<std::collections::HashMap<_, _>>();
        (view, paths)
    };
    let View::List { list } = &mut view else {
        unreachable!()
    };

    // Render only complete cached artifacts; native acquisition stays on the icon worker.
    let selected = list.selected_item_id.clone();
    let mut selected_references = Vec::new();
    if let Some(paths) = selected.as_ref().and_then(|selected| paths.get(selected)) {
        for path in paths.iter().take(FILE_COLLECTION_PREVIEW_LIMIT) {
            let path = Path::new(path);
            selected_references.push(icon_for_path(path));
        }
    }
    for item in list
        .sections
        .iter_mut()
        .flat_map(|section| &mut section.items)
    {
        let reference = if selected.as_ref() == Some(&item.id) {
            selected_references.first().cloned().flatten().flatten()
        } else {
            paths
                .get(&item.id)
                .and_then(|paths| paths.first())
                .and_then(|path| icon_for_path(Path::new(path)))
                .flatten()
        };
        if let Some(reference) = reference {
            item.icon = Some(ViewItemIcon::Native(reference));
        }
    }
    if let Some(DetailView {
        content: DetailContent::Files { files },
        ..
    }) = &mut list.detail
        && selected_references.iter().all(Option::is_some)
    {
        // Publish the stack together, settling failures with semantic icons; list icons remain progressive.
        for (file, reference) in files.iter_mut().zip(selected_references) {
            file.icon = reference.flatten();
        }
    }
    view
}

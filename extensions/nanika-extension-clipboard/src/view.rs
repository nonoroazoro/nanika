use nanika_protocol::{
    ClipboardContent, DetailView, ListItem, ListLayout, ListSection, ListView, View, ViewAction,
    ViewActionStyle, ViewFilter, ViewFilterOption, ViewMetadata,
};

use crate::{COPY_ACTION_ID, ClipboardEntry, ClipboardViewState};

pub fn clipboard_view(state: &mut ClipboardViewState, entries: &[ClipboardEntry]) -> View {
    let matching = entries
        .iter()
        .filter(|entry| matches_content_type(entry, &state.content_type))
        .filter(|entry| matches_query(entry, &state.query))
        .collect::<Vec<_>>();
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
    let pinned = visible
        .iter()
        .filter(|entry| entry.pinned)
        .map(|entry| list_item(entry))
        .collect::<Vec<_>>();
    let recent = visible
        .iter()
        .filter(|entry| !entry.pinned)
        .map(|entry| list_item(entry))
        .collect::<Vec<_>>();
    let mut sections = Vec::with_capacity(2);
    if !pinned.is_empty() {
        sections.push(ListSection {
            id: "pinned".to_owned(),
            title: Some("Pinned".to_owned()),
            items: pinned,
        });
    }
    if !recent.is_empty() {
        sections.push(ListSection {
            id: "recent".to_owned(),
            title: Some("Recent".to_owned()),
            items: recent,
        });
    }
    View::List {
        list: Box::new(ListView {
            title: "Clipboard History".to_owned(),
            search_placeholder: "Filter clipboard history".to_owned(),
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

fn list_item(entry: &ClipboardEntry) -> ListItem {
    ListItem {
        id: entry.entry_id.clone(),
        title: entry.title.clone(),
        subtitle: Some(content_type(entry).to_owned()),
        actions: vec![copy_action()],
    }
}

fn detail_view(entry: &ClipboardEntry) -> DetailView {
    DetailView {
        title: Some(entry.title.clone()),
        body: match &entry.content {
            ClipboardContent::Text { value } => value.clone(),
            ClipboardContent::Files { paths } => paths.join("\n"),
            ClipboardContent::PngFile { .. } => "Image clipboard content".to_owned(),
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

fn copy_action() -> ViewAction {
    ViewAction {
        id: COPY_ACTION_ID.to_owned(),
        title: "Copy to Clipboard".to_owned(),
        style: ViewActionStyle::Primary,
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
        || std::iter::once(entry.title.as_str())
            .chain(searchable_values(entry).iter().map(String::as_str))
            .any(|value| value.to_lowercase().contains(&query))
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

fn searchable_values(entry: &ClipboardEntry) -> Vec<String> {
    match &entry.content {
        ClipboardContent::Text { value } => vec![value.chars().take(2_048).collect()],
        ClipboardContent::Files { paths } => paths.iter().take(32).cloned().collect(),
        ClipboardContent::PngFile { .. } => Vec::new(),
    }
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

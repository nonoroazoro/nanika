use std::path::Path;

use nanika_protocol::{
    Action, ActionStyle, ClipboardContent, DetailContent, DetailView, ImageSource, ListItem,
    ListLayout, ListSection, ListView, View, ViewFile, ViewFilter, ViewFilterOption, ViewItemIcon,
    ViewMetadata,
};

use crate::{CLEAR_ACTION_ID, COPY_ACTION_ID, ClipboardEntry, ClipboardViewState};

pub const FILE_COLLECTION_PREVIEW_LIMIT: usize = 3;

pub(crate) fn clipboard_view(
    state: &mut ClipboardViewState,
    visible: &[crate::ClipboardItem],
    selected: Option<&ClipboardEntry>,
    total: usize,
) -> Result<View, String> {
    let items = visible.iter().map(list_item).collect::<Result<_, _>>()?;
    let sections = (!visible.is_empty())
        .then(|| ListSection {
            id: "all".to_owned(),
            title: None,
            items,
        })
        .into_iter()
        .collect();
    Ok(View::List {
        list: Box::new(ListView {
            title: "Clipboard History".to_owned(),
            search_placeholder: "Search for entries".to_owned(),
            search_text: state.query.clone(),
            layout: ListLayout::Split,
            sections,
            selected_item_id: state.selected_item_id.clone(),
            detail: selected
                .map(|entry| detail_view(entry, state.text_offset))
                .transpose()?,
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
            pagination: _pagination(state.page_offset, crate::CLIPBOARD_PAGE_SIZE, total),
        }),
    })
}
fn list_item(entry: &crate::ClipboardItem) -> Result<ListItem, String> {
    Ok(ListItem {
        id: entry.entry_id.clone(),
        title: crate::labels::display_label(&entry.title, 128),
        subtitle: None,
        icon: Some(match entry.kind.as_str() {
            "text" => ViewItemIcon::Text,
            "files" => ViewItemIcon::Files,
            "image" => ViewItemIcon::Image,
            _ => return Err("invalid clipboard content kind".into()),
        }),
        actions: vec![clear_action(), copy_action()],
    })
}

fn detail_view(entry: &ClipboardEntry, text_offset: usize) -> Result<DetailView, String> {
    Ok(DetailView {
        title: None,
        content: match &entry.content {
            ClipboardContent::Text { value } => {
                const TEXT_PAGE_SIZE: usize = nanika_protocol::MAX_DETAIL_TEXT_CHARS;
                let total = value.chars().count();
                let offset =
                    text_offset.min(total.saturating_sub(1) / TEXT_PAGE_SIZE * TEXT_PAGE_SIZE);
                DetailContent::Text {
                    value: value.chars().skip(offset).take(TEXT_PAGE_SIZE).collect(),
                    pagination: _pagination(offset, TEXT_PAGE_SIZE, total).map(Box::new),
                }
            }
            ClipboardContent::Files { paths } => DetailContent::Files {
                files: paths
                    .iter()
                    .map(|path| ViewFile {
                        path: path.clone(),
                        icon: None,
                        name: crate::labels::display_label(
                            Path::new(path)
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or(path),
                            128,
                        ),
                    })
                    .collect(),
            },
            ClipboardContent::PngFile { .. } => {
                if !nanika_protocol::is_valid_content_hash(&entry.entry_id) {
                    return Err("clipboard image identity is invalid".into());
                }
                DetailContent::Image {
                    source: ImageSource {
                        path: format!("{}.png", entry.entry_id),
                    },
                    alternative_text: crate::labels::display_label(&entry.title, 128),
                }
            }
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
    })
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

fn _pagination(
    offset: usize,
    size: usize,
    total: usize,
) -> Option<nanika_protocol::ViewPagination> {
    (total > size).then(|| nanika_protocol::ViewPagination {
        label: format!("{}-{} of {}", offset + 1, (offset + size).min(total), total),
        previous_cursor: offset.checked_sub(size).map(|offset| offset.to_string()),
        next_cursor: (offset + size < total).then(|| (offset + size).to_string()),
    })
}

/// Accept only an adjacent cursor from the currently rendered data scope.
pub fn change_page(
    state: &mut ClipboardViewState,
    current: &View,
    target: nanika_protocol::ViewPageTarget,
    cursor: &str,
) -> Result<(), String> {
    let View::List { list } = current else {
        return Err("Clipboard view is not a list.".into());
    };
    let pagination = match target {
        nanika_protocol::ViewPageTarget::List => list.pagination.as_ref(),
        nanika_protocol::ViewPageTarget::Detail => {
            list.detail
                .as_ref()
                .and_then(|detail| match &detail.content {
                    DetailContent::Text { pagination, .. } => pagination.as_deref(),
                    _ => None,
                })
        }
    };
    if !pagination.is_some_and(|page| page.allows(cursor)) {
        return Err("The requested page is no longer available.".to_owned());
    }
    let offset = cursor.parse::<usize>().map_err(|error| error.to_string())?;
    match target {
        nanika_protocol::ViewPageTarget::List => {
            state.page_offset = offset;
            state.selected_item_id = None;
            state.text_offset = 0;
        }
        nanika_protocol::ViewPageTarget::Detail => state.text_offset = offset,
    }
    Ok(())
}

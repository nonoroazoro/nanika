use std::path::Path;

use nanika_protocol::{
    Action, ActionStyle, ClipboardContent, DetailContent, DetailView, ImageSource, ListItem,
    ListLayout, ListSection, ListView, View, ViewFile, ViewFilter, ViewFilterOption, ViewItemIcon,
    ViewMetadata,
};

use crate::{CLEAR_ACTION_ID, COPY_ACTION_ID, ClipboardEntry, ClipboardViewState};

pub const FILE_COLLECTION_PREVIEW_LIMIT: usize = 3;

pub(crate) fn clipboard_view(
    state: &ClipboardViewState,
    visible: &[crate::ClipboardItem],
    selected: Option<&crate::ClipboardPreview>,
    selected_index: Option<usize>,
    total: usize,
    collection_id: String,
) -> Result<View, String> {
    let filtered = !state.query.trim().is_empty() || state.content_type != "all";
    let items = visible.iter().map(list_item).collect::<Result<_, _>>()?;
    let sections = vec![ListSection {
        id: "all".to_owned(),
        title: None,
        offset: state.offset,
        total,
        items,
    }];
    Ok(View::List {
        list: Box::new(ListView {
            title: "Clipboard History".to_owned(),
            search_placeholder: "Search clipboard history".to_owned(),
            search_text: state.query.clone(),
            empty_title: if filtered {
                "No matching entries"
            } else {
                "No clipboard history yet"
            }
            .to_owned(),
            empty_description: if filtered {
                "Try a different search or filter."
            } else {
                "Copied content will appear here."
            }
            .to_owned(),
            layout: ListLayout::Split,
            sections,
            collection_id,
            selection: selected
                .zip(selected_index)
                .map(|(preview, index)| {
                    list_item(&crate::ClipboardItem::from_entry(&preview.entry))
                        .map(|item| nanika_protocol::ListSelection { index, item })
                })
                .transpose()?,
            detail: selected
                .map(|entry| detail_view(entry, state.text_chunk))
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

fn detail_view(preview: &crate::ClipboardPreview, chunk: usize) -> Result<DetailView, String> {
    let entry = &preview.entry;
    Ok(DetailView {
        title: None,
        content: match &entry.content {
            ClipboardContent::Text { value } => {
                let range = preview
                    .chunks
                    .get(chunk)
                    .ok_or("Text chunk is outside its document.")?;
                DetailContent::Text {
                    value: value[range.clone()].to_owned(),
                    text_id: entry.entry_id.clone(),
                    chunk_index: chunk,
                    total_chunks: preview.chunks.len(),
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
        icon: None,
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
        icon: None,
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

/// Range requests address an immutable collection identity, independent of selection updates.
pub fn read_range(
    state: &mut ClipboardViewState,
    current: &View,
    collection_id: &str,
    offset: usize,
    count: std::num::NonZeroU32,
) -> Result<(), String> {
    let View::List { list } = current else {
        return Err("Clipboard view is not a list.".into());
    };
    if collection_id != list.collection_id
        || count.get() as usize > nanika_protocol::MAX_VIEW_ITEMS
        || offset > list.total()
        || (offset == list.total() && offset != 0)
    {
        return Err("The requested collection range is no longer available.".into());
    }
    state.offset = offset;
    state.count = count.get() as usize;
    state.anchor_id = None;
    Ok(())
}

pub fn read_text_chunk(
    state: &mut ClipboardViewState,
    current: &View,
    text_id: &str,
    index: usize,
) -> Result<(), String> {
    let View::List { list } = current else {
        return Err("Clipboard view is not a list.".into());
    };
    let Some(DetailView {
        content:
            DetailContent::Text {
                text_id: current_id,
                total_chunks,
                ..
            },
        ..
    }) = &list.detail
    else {
        return Err("Clipboard detail is not text.".into());
    };
    if text_id != current_id || index >= *total_chunks {
        return Err("Text document changed.".into());
    }
    state.text_chunk = index;
    Ok(())
}

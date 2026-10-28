use std::path::Path;

use nanika_protocol::{
    Action, ActionStyle, ClipboardContent, DetailContent, DetailView, ImageSource, ListItem,
    ListLayout, ListSection, ListView, View, ViewFile, ViewFilter, ViewFilterOption, ViewItemIcon,
    ViewMetadata
};

use crate::{COPY_ACTION_ID, ClipboardEntry, ClipboardViewState};

pub const FILE_COLLECTION_PREVIEW_LIMIT: usize = 3;

pub(crate) fn clipboard_view(
    syntax_highlighting: bool,
    state: &ClipboardViewState,
    visible: &[crate::ClipboardItem],
    selected: Option<&crate::ClipboardPreview>,
    selected_index: Option<usize>,
    total: usize,
    collection_id: String
) -> Result<View, String> {
    let filtered = !state.query.trim().is_empty() || state.content_type != "all";
    let items = visible.iter().map(list_item).collect::<Result<_, _>>()?;
    let sections = vec![ListSection {
        id: "all".to_owned(),
        title: None,
        offset: state.offset,
        total,
        items
    }];
    Ok(View::List {
        list: Box::new(ListView {
            title: nanika_protocol::message("extension-name", []),
            search_placeholder: nanika_protocol::message("view-search-placeholder", []),
            search_text: state.query.clone(),
            empty_title: nanika_protocol::message(
                if filtered {
                    "view-empty-filtered-title"
                } else {
                    "view-empty-title"
                },
                []
            ),
            empty_description: nanika_protocol::message(
                if filtered {
                    "view-empty-filtered-description"
                } else {
                    "view-empty-description"
                },
                []
            ),
            layout: ListLayout::Split,
            sections,
            collection_id,
            selection: selected
                .zip(selected_index)
                .map(|(preview, index)| {
                    list_item(&crate::ClipboardItem::from_entry(&preview.entry)?)
                        .map(|item| nanika_protocol::ListSelection { index, item })
                })
                .transpose()?,
            detail: selected
                .map(|entry| detail_view(entry, syntax_highlighting))
                .transpose()?,
            filter: Some(ViewFilter {
                id: "contentType".to_owned(),
                selected_value: state.content_type.clone(),
                options: vec![
                    filter_option("all", "filter-all"),
                    filter_option("text", "content-text"),
                    filter_option("files", "content-files"),
                    filter_option("images", "content-images"),
                ]
            })
        })
    })
}
fn list_item(entry: &crate::ClipboardItem) -> Result<ListItem, String> {
    Ok(ListItem {
        id: entry.entry_id.clone(),
        title: entry.summary.title()?,
        subtitle: None,
        icon: Some(match entry.summary {
            crate::ClipboardSummary::Text { .. } => ViewItemIcon::Text,
            crate::ClipboardSummary::Files { .. } => ViewItemIcon::Files,
            crate::ClipboardSummary::Image { .. } => ViewItemIcon::Image
        }),
        actions: vec![copy_action()]
    })
}

fn detail_view(
    preview: &crate::ClipboardPreview,
    syntax_highlighting: bool
) -> Result<DetailView, String> {
    let entry = &preview.entry;
    Ok(DetailView {
        title: None,
        content: match &entry.content {
            ClipboardContent::Text { .. } => {
                let document = &preview
                    .document
                    .as_ref()
                    .ok_or("Text index is unavailable.")?
                    .document;
                let first = preview.read(&nanika_protocol::TextRead {
                    document_id: document.id.clone(),
                    revision: document.revision.clone(),
                    index: 0
                })?;
                DetailContent::Text {
                    value: first.value,
                    document: document.clone(),
                    syntax_highlighting
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
                            128
                        )
                    })
                    .collect()
            },
            ClipboardContent::PngFile { .. } => {
                if !nanika_protocol::is_valid_content_hash(&entry.entry_id) {
                    return Err("clipboard image identity is invalid".into());
                }
                DetailContent::Image {
                    source: ImageSource {
                        path: format!("{}.png", entry.entry_id)
                    },
                    alternative_text: entry.summary()?.title()?
                }
            }
        },
        metadata: vec![
            ViewMetadata {
                title: nanika_protocol::message("metadata-content-type", []),
                value: nanika_protocol::message(content_type(entry), []).into()
            },
            ViewMetadata {
                title: nanika_protocol::message("metadata-size", []),
                value: nanika_protocol::ViewMetadataValue::Bytes {
                    bytes: entry.byte_size
                }
            },
        ],
        actions: Vec::new()
    })
}

fn copy_action() -> Action {
    Action {
        shortcut: None,
        icon: None,
        id: COPY_ACTION_ID.to_owned(),
        title: nanika_protocol::message("action-copy", []),
        confirmation_description: None,
        confirmation_title: None,
        allow_default_execution: true,
        style: ActionStyle::Primary,
        enabled: true,
        group: None
    }
}

fn filter_option(value: &str, title: &str) -> ViewFilterOption {
    ViewFilterOption {
        value: value.to_owned(),
        title: nanika_protocol::message(title, [])
    }
}

fn content_type(entry: &ClipboardEntry) -> &'static str {
    match entry.content {
        ClipboardContent::Text { .. } => "content-text",
        ClipboardContent::Files { .. } => "content-files",
        ClipboardContent::PngFile { .. } => "content-image"
    }
}

/// Range requests address an immutable collection identity, independent of selection updates.
pub fn read_range(
    state: &mut ClipboardViewState,
    current: &View,
    collection_id: &str,
    offset: usize,
    count: std::num::NonZeroU32
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

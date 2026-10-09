use serde::{Deserialize, Serialize};

use std::collections::HashSet;

use crate::{Action, ActionStyle, DetailContent, DetailView, ListView};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum View {
    List { list: Box<ListView> },
    Detail { detail: DetailView }
}

impl View {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::List { list } => validate_list(list),
            Self::Detail { detail } => validate_detail(detail)
        }
    }
}

fn validate_list(list: &ListView) -> Result<(), String> {
    validate_text("view title", &list.title, 256, false)?;
    validate_text(
        "view search placeholder",
        &list.search_placeholder,
        256,
        true
    )?;
    validate_view_search_text(&list.search_text)?;
    validate_text("view empty title", &list.empty_title, 256, false)?;
    validate_text("view empty description", &list.empty_description, 512, true)?;
    if list.sections.len() > 32 {
        return Err("view has too many list sections".to_owned());
    }
    validate_id("view collection id", &list.collection_id)?;
    let mut total = 0usize;
    let mut delivered = 0usize;
    let mut section_ids = HashSet::new();
    let mut item_ids = HashSet::new();
    for section in &list.sections {
        validate_id("list section id", &section.id)?;
        if !section_ids.insert(section.id.as_str()) {
            return Err("view list section ids must be unique".to_owned());
        }
        if let Some(title) = &section.title {
            validate_text("list section title", title, 256, false)?;
        }
        total = total
            .checked_add(section.total)
            .filter(|total| *total <= u32::MAX as usize)
            .ok_or("list total exceeds the supported range")?;
        delivered += section.items.len();
        if section.offset > section.total || section.items.len() > section.total - section.offset {
            return Err("list section window is outside its collection".into());
        }
        for item in &section.items {
            validate_item(item)?;
            if !item_ids.insert(item.id.as_str()) {
                return Err("view list item ids must be unique".to_owned());
            }
        }
    }
    if delivered > crate::MAX_VIEW_ITEMS {
        return Err("view has too many delivered list items".into());
    }
    if let Some(selection) = &list.selection {
        validate_item(&selection.item)?;
        if selection.index >= total {
            return Err("view selection is outside its collection".into());
        }
        let mut base = 0;
        for section in &list.sections {
            for (local, item) in section.items.iter().enumerate() {
                let index = base + section.offset + local;
                if (index == selection.index || item.id == selection.item.id)
                    && (index != selection.index || item != &selection.item)
                {
                    return Err("view selection disagrees with its delivered row".into());
                }
            }
            base += section.total;
        }
    }
    if let Some(detail) = &list.detail {
        validate_detail(detail)?;
        if !detail.actions.is_empty() {
            return Err(
                "list detail actions must be declared on the selected list item".to_owned()
            );
        }
    }
    if let Some(filter) = &list.filter {
        validate_id("view filter id", &filter.id)?;
        if filter.options.is_empty() || filter.options.len() > 32 {
            return Err("view filter option count is invalid".to_owned());
        }
        let mut values = HashSet::new();
        for option in &filter.options {
            validate_id("view filter option value", &option.value)?;
            validate_text("view filter option title", &option.title, 128, false)?;
            if !values.insert(option.value.as_str()) {
                return Err("view filter option values must be unique".to_owned());
            }
        }
        if !values.contains(filter.selected_value.as_str()) {
            return Err("view filter selection is invalid".to_owned());
        }
    }
    Ok(())
}

fn validate_detail(detail: &DetailView) -> Result<(), String> {
    if let Some(title) = &detail.title {
        validate_text("detail title", title, 512, true)?;
    }
    match &detail.content {
        DetailContent::Text {
            value,
            text_id,
            chunk_index,
            total_chunks,
        } => {
            validate_id("detail text identity", text_id)?;
            if *total_chunks == 0
                || *total_chunks > crate::MAX_DETAIL_TEXT_CHUNKS
                || chunk_index >= total_chunks
                || value.chars().count() > crate::DETAIL_TEXT_BATCH_CHARS
                || (*total_chunks > 1 && value.is_empty())
            {
                return Err("detail text chunk is invalid".into());
            }
        }
        DetailContent::Files { files } => {
            if files.is_empty() || files.len() > 256 {
                return Err("detail file count is invalid".to_owned());
            }
            if files.iter().map(|file| file.path.len()).sum::<usize>() > 1024 * 1024 {
                return Err("detail file paths exceed the supported size".to_owned());
            }
            for file in files {
                validate_text("detail file name", &file.name, 512, false)?;
                if file.path.is_empty() || file.path.len() > 1024 * 1024 || file.path.contains('\0')
                {
                    return Err("detail file path is invalid".to_owned());
                }
                if file
                    .icon
                    .as_ref()
                    .is_some_and(|reference| !reference.is_valid())
                {
                    return Err("detail file icon reference is invalid".to_owned());
                }
            }
        }
        DetailContent::Image {
            source,
            alternative_text
        } => {
            validate_text(
                "detail image alternative text",
                alternative_text,
                512,
                false
            )?;
            if !crate::is_valid_resource_path(&source.path) {
                return Err("detail image resource path is invalid".to_owned());
            }
        }
    }
    if detail.metadata.len() > 64 {
        return Err("view detail has too much metadata".to_owned());
    }
    for metadata in &detail.metadata {
        validate_text("detail metadata title", &metadata.title, 256, false)?;
        validate_text("detail metadata value", &metadata.value, 2_048, true)?;
    }
    validate_actions(&detail.actions)
}

pub fn validate_actions(actions: &[Action]) -> Result<(), String> {
    if actions.len() > 16 {
        return Err("view exposes too many actions".to_owned());
    }
    let mut ids = HashSet::new();
    for action in actions {
        if let Some(group) = &action.group {
            validate_id("action group", group)?;
        }
        validate_id("view action id", &action.id)?;
        validate_text("view action title", &action.title, 128, false)?;
        if let Some(title) = &action.confirmation_title {
            if action.allow_default_execution {
                return Err(
                    "action requiring confirmation cannot allow default execution".to_owned()
                );
            }
            validate_text("view action confirmation title", title, 128, false)?;
            if action.style != ActionStyle::Destructive {
                return Err("view action confirmation requires destructive style".to_owned());
            }
        }
        if !ids.insert(action.id.as_str()) {
            return Err("view action ids must be unique".to_owned());
        }
    }
    Ok(())
}

fn validate_id(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return Err(format!("{field} is invalid"));
    }
    Ok(())
}

fn validate_text(
    field: &str,
    value: &str,
    maximum_chars: usize,
    allow_empty: bool
) -> Result<(), String> {
    if (!allow_empty && value.trim().is_empty())
        || value.chars().count() > maximum_chars
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(format!("{field} is invalid"));
    }
    Ok(())
}

fn validate_item(item: &crate::ListItem) -> Result<(), String> {
    validate_id("list item id", &item.id)?;
    validate_text("list item title", &item.title, 512, false)?;
    if let Some(subtitle) = &item.subtitle {
        validate_text("list item subtitle", subtitle, 512, true)?;
    }
    validate_actions(&item.actions)?;
    if let Some(crate::ViewItemIcon::Native(reference)) = &item.icon
        && !reference.is_valid()
    {
        return Err("list item icon reference is invalid".into());
    }
    Ok(())
}

/// Search input is plain user data; control characters do not become markup or labels.
pub fn validate_view_search_text(value: &str) -> Result<(), String> {
    if value.chars().count() > 4096 {
        return Err("view search text exceeds the supported size".to_owned());
    }
    Ok(())
}

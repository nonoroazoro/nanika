use serde::{Deserialize, Serialize};

use std::collections::HashSet;

use crate::{Action, ActionStyle, DetailContent, DetailView, ImageSource, ListView};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum View {
    List { list: Box<ListView> },
    Detail { detail: DetailView },
}

impl View {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::List { list } => validate_list(list),
            Self::Detail { detail } => validate_detail(detail),
        }
    }
}

fn validate_list(list: &ListView) -> Result<(), String> {
    validate_text("view title", &list.title, 256, false)?;
    validate_text(
        "view search placeholder",
        &list.search_placeholder,
        256,
        true,
    )?;
    validate_text("view search text", &list.search_text, 4_096, true)?;
    if list.sections.len() > 32 {
        return Err("view has too many list sections".to_owned());
    }
    let mut item_ids = HashSet::new();
    let mut item_count = 0_usize;
    for section in &list.sections {
        validate_id("list section id", &section.id)?;
        if let Some(title) = &section.title {
            validate_text("list section title", title, 256, false)?;
        }
        item_count = item_count.saturating_add(section.items.len());
        for item in &section.items {
            validate_id("list item id", &item.id)?;
            if !item_ids.insert(item.id.as_str()) {
                return Err("view list item ids must be unique".to_owned());
            }
            validate_text("list item title", &item.title, 512, false)?;
            if let Some(subtitle) = &item.subtitle {
                validate_text("list item subtitle", subtitle, 512, true)?;
            }
            validate_actions(&item.actions)?;
            if let Some(crate::ViewItemIcon::Native(reference)) = &item.icon
                && !reference.is_valid()
            {
                return Err("list item icon reference is invalid".to_owned());
            }
        }
    }
    if item_count > 500 {
        return Err("view has too many list items".to_owned());
    }
    if let Some(selected) = &list.selected_item_id
        && !item_ids.contains(selected.as_str())
    {
        return Err("view selection does not reference a list item".to_owned());
    }
    if let Some(detail) = &list.detail {
        validate_detail(detail)?;
        if !detail.actions.is_empty() {
            return Err(
                "list detail actions must be declared on the selected list item".to_owned(),
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
    if let Some(cursor) = &list.next_cursor {
        validate_text("view pagination cursor", cursor, 512, false)?;
    }
    Ok(())
}

fn validate_detail(detail: &DetailView) -> Result<(), String> {
    if let Some(title) = &detail.title {
        validate_text("detail title", title, 512, true)?;
    }
    match &detail.content {
        DetailContent::Text { value } => {
            validate_text("detail text", value, 262_144, true)?;
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
                validate_text("detail file path", &file.path, 1024 * 1024, false)?;
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
            alternative_text,
        } => {
            validate_text(
                "detail image alternative text",
                alternative_text,
                512,
                false,
            )?;
            match source {
                ImageSource::DataUrl { value } => {
                    const MAX_IMAGE_DATA_URL_CHARS: usize = 24 * 1024 * 1024;
                    if value.chars().count() > MAX_IMAGE_DATA_URL_CHARS
                        || !value.starts_with("data:image/")
                        || !value.contains(";base64,")
                    {
                        return Err("detail image data is invalid or too large".to_owned());
                    }
                }
                ImageSource::Resource { path } => {
                    if !crate::is_valid_resource_path(path) {
                        return Err("detail image resource path is invalid".to_owned());
                    }
                }
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
                    "action requiring confirmation cannot allow default execution".to_owned(),
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
    allow_empty: bool,
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

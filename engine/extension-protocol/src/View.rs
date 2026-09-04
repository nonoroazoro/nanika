use serde::{Deserialize, Serialize};

use std::collections::HashSet;

use crate::{DetailView, ListView, ViewAction};

/// One host-rendered extension view document.
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
    validate_text("detail body", &detail.body, 262_144, true)?;
    if detail.metadata.len() > 64 {
        return Err("view detail has too much metadata".to_owned());
    }
    for metadata in &detail.metadata {
        validate_text("detail metadata title", &metadata.title, 256, false)?;
        validate_text("detail metadata value", &metadata.value, 2_048, true)?;
    }
    validate_actions(&detail.actions)
}

fn validate_actions(actions: &[ViewAction]) -> Result<(), String> {
    if actions.len() > 16 {
        return Err("view exposes too many actions".to_owned());
    }
    let mut ids = HashSet::new();
    for action in actions {
        validate_id("view action id", &action.id)?;
        validate_text("view action title", &action.title, 128, false)?;
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

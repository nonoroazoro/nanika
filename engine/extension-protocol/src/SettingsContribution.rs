use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    MAX_FRAME_BYTES, SettingColumn, SettingColumnControl, SettingControl, SettingField,
    SettingValue,
};

const MAX_SETTINGS_BYTES: usize = MAX_FRAME_BYTES / 2;

/// Bounded settings schema and current values contributed by one extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsContribution {
    pub title: String,
    pub fields: Vec<SettingField>,
}

impl SettingsContribution {
    pub fn validate(&self) -> Result<(), String> {
        validate_required_text(&self.title, 128, "settings title")?;
        if self.fields.len() > 64 {
            return Err("settings contribution exceeds 64 fields".to_owned());
        }
        let mut keys = HashSet::with_capacity(self.fields.len());
        for field in &self.fields {
            validate_key(&field.key)?;
            if !keys.insert(&field.key) {
                return Err(format!("duplicate setting key: {}", field.key));
            }
            validate_required_text(&field.title, 128, "setting title")?;
            if let Some(description) = &field.description {
                validate_text(description, 512, "setting description")?;
            }
            validate_field(field)?;
        }
        let encoded = serde_json::to_vec(self)
            .map_err(|error| format!("settings contribution cannot be encoded: {error}"))?;
        if encoded.len() > MAX_SETTINGS_BYTES {
            return Err(format!(
                "settings contribution exceeds {MAX_SETTINGS_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

fn validate_field(field: &SettingField) -> Result<(), String> {
    match (&field.control, &field.value) {
        (SettingControl::Toggle, SettingValue::Boolean { .. }) => Ok(()),
        (SettingControl::Text { placeholder, .. }, SettingValue::Text { value }) => {
            validate_optional_placeholder(placeholder)?;
            validate_text(value, 4_096, "setting value")
        }
        (
            SettingControl::StringList {
                placeholder,
                max_items,
                ..
            },
            SettingValue::StringList { values },
        ) => {
            validate_optional_placeholder(placeholder)?;
            validate_string_list(values, *max_items)
        }
        (SettingControl::RecordTable { columns, max_rows }, SettingValue::Records { rows }) => {
            validate_records(columns, *max_rows, rows)
        }
        _ => Err(format!(
            "setting control and value do not match: {}",
            field.key
        )),
    }
}

fn validate_records(
    columns: &[SettingColumn],
    max_rows: u32,
    rows: &[std::collections::BTreeMap<String, SettingValue>],
) -> Result<(), String> {
    if columns.is_empty() || columns.len() > 16 {
        return Err("record table must contain between 1 and 16 columns".to_owned());
    }
    if max_rows == 0 || max_rows > 5_000 || rows.len() > max_rows as usize {
        return Err("record table exceeds its row limit".to_owned());
    }
    let mut column_keys = HashSet::with_capacity(columns.len());
    for column in columns {
        validate_key(&column.key)?;
        validate_required_text(&column.title, 128, "column title")?;
        if !column_keys.insert(column.key.as_str()) {
            return Err(format!("duplicate record column: {}", column.key));
        }
        match &column.control {
            SettingColumnControl::Text { placeholder, .. }
            | SettingColumnControl::StringList { placeholder, .. } => {
                validate_optional_placeholder(placeholder)?;
            }
        }
    }
    for row in rows {
        if row.len() != columns.len() {
            return Err("record row does not match its columns".to_owned());
        }
        for column in columns {
            let value = row
                .get(&column.key)
                .ok_or_else(|| format!("record row is missing {}", column.key))?;
            match (&column.control, value) {
                (SettingColumnControl::Text { .. }, SettingValue::Text { value }) => {
                    validate_text(value, 4_096, "record value")?;
                    if column.required && value.trim().is_empty() {
                        return Err(format!("record value is required: {}", column.key));
                    }
                }
                (
                    SettingColumnControl::StringList { max_items, .. },
                    SettingValue::StringList { values },
                ) => validate_string_list(values, *max_items)?,
                _ => return Err(format!("record value type is invalid: {}", column.key)),
            }
        }
    }
    Ok(())
}

fn validate_string_list(values: &[String], max_items: u32) -> Result<(), String> {
    if max_items == 0 || max_items > 256 || values.len() > max_items as usize {
        return Err("setting list exceeds its item limit".to_owned());
    }
    for value in values {
        validate_text(value, 4_096, "setting list item")?;
    }
    Ok(())
}

fn validate_optional_placeholder(value: &Option<String>) -> Result<(), String> {
    if let Some(value) = value {
        validate_text(value, 128, "setting placeholder")?;
    }
    Ok(())
}

fn validate_key(value: &str) -> Result<(), String> {
    let mut characters = value.chars();
    if !matches!(characters.next(), Some('a'..='z'))
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return Err(format!("invalid setting key: {value}"));
    }
    if value.len() > 128 {
        return Err("setting key exceeds 128 bytes".to_owned());
    }
    Ok(())
}

fn validate_text(value: &str, maximum: usize, label: &str) -> Result<(), String> {
    if value.len() > maximum {
        Err(format!("{label} exceeds {maximum} bytes"))
    } else {
        Ok(())
    }
}

fn validate_required_text(value: &str, maximum: usize, label: &str) -> Result<(), String> {
    validate_text(value, maximum, label)?;
    if value.trim().is_empty() {
        Err(format!("{label} is empty"))
    } else {
        Ok(())
    }
}

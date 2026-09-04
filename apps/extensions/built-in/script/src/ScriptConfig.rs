use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use nanika_config::ConfigStore;
use nanika_protocol::{
    SettingColumn, SettingColumnControl, SettingControl, SettingField, SettingUpdate, SettingValue,
    SettingsContribution,
};
use serde::{Deserialize, Serialize};

use crate::{EXTENSION_ID, ScriptEntry};

const FORMAT_VERSION: u32 = 1;
const MAX_SCRIPTS: usize = 5_000;
const MAX_TEXT_BYTES: usize = 4_096;

/// Human-edited script extension settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptConfig {
    pub format_version: u32,
    #[serde(default)]
    pub scripts: Vec<ScriptEntry>,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            scripts: Vec::new(),
        }
    }
}

impl ScriptConfig {
    pub fn load(store: &ConfigStore) -> Result<Self, String> {
        let path = Self::path(store.config_root());
        if !path.exists() {
            return Ok(Self::default());
        }
        if !path.is_file() {
            return Err(format!(
                "script settings path is not a file: {}",
                path.display()
            ));
        }
        let config = store
            .load::<Self>(&path)
            .map_err(|error| error.to_string())?;
        config.validate()?;
        Ok(config)
    }

    pub fn path(config_root: &Path) -> PathBuf {
        config_root
            .join("extensions")
            .join(EXTENSION_ID)
            .join("settings.jsonc")
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "unsupported script settings format {}",
                self.format_version
            ));
        }
        if self.scripts.len() > MAX_SCRIPTS {
            return Err(format!(
                "script settings exceed the {MAX_SCRIPTS} entry limit"
            ));
        }
        let mut ids = HashSet::with_capacity(self.scripts.len());
        for script in &self.scripts {
            if !valid_id(&script.id) || script.id.len() > 128 || !ids.insert(script.id.as_str()) {
                return Err(format!("invalid or duplicate script id: {}", script.id));
            }
            if script.title.trim().is_empty()
                || !bounded_text(&script.title)
                || script.aliases.len() > 64
                || script.aliases.iter().any(|value| !bounded_text(value))
                || script.arguments.len() > 256
                || script.arguments.iter().any(|value| !bounded_text(value))
                || !script.interpreter.is_absolute()
                || !bounded_path(&script.interpreter)
                || !script.script.is_absolute()
                || !bounded_path(&script.script)
                || script
                    .working_directory
                    .as_ref()
                    .is_some_and(|path| !path.is_absolute() || !bounded_path(path))
            {
                return Err(format!("invalid script settings: {}", script.id));
            }
        }
        self.settings().validate()?;
        Ok(())
    }

    pub fn settings(&self) -> SettingsContribution {
        SettingsContribution {
            title: "Scripts".to_owned(),
            fields: vec![SettingField {
                key: "scripts".to_owned(),
                title: "Scripts".to_owned(),
                description: None,
                control: SettingControl::RecordTable {
                    columns: script_columns(),
                    max_rows: MAX_SCRIPTS as u32,
                },
                value: SettingValue::Records {
                    rows: self.scripts.iter().map(script_row).collect(),
                },
            }],
        }
    }

    pub fn update(&self, store: &ConfigStore, updates: Vec<SettingUpdate>) -> Result<Self, String> {
        if updates.len() != 1 || updates[0].key != "scripts" {
            return Err("script settings require exactly one scripts update".to_owned());
        }
        let SettingValue::Records { rows } = &updates[0].value else {
            return Err("script settings have an invalid value".to_owned());
        };
        let next = Self {
            format_version: FORMAT_VERSION,
            scripts: rows.iter().map(script_from_row).collect::<Result<_, _>>()?,
        };
        next.validate()?;
        let path = Self::path(store.config_root());
        let mut changed = Vec::with_capacity(2);
        if !path.is_file() {
            changed.push((
                "formatVersion".to_owned(),
                serde_json::json!(FORMAT_VERSION),
            ));
        }
        changed.push((
            "scripts".to_owned(),
            serde_json::to_value(&next.scripts).map_err(|error| error.to_string())?,
        ));
        store
            .update::<Self>(&path, changed, Self::validate)
            .map_err(|error| error.to_string())
    }
}

fn bounded_text(value: &str) -> bool {
    value.len() <= MAX_TEXT_BYTES
}

fn bounded_path(path: &Path) -> bool {
    path.to_string_lossy().len() <= MAX_TEXT_BYTES
}

fn script_columns() -> Vec<SettingColumn> {
    vec![
        text_column("id", "ID", false, true),
        text_column("title", "Title", false, true),
        list_column("aliases", "Aliases", 64),
        text_column("interpreter", "Interpreter", true, true),
        text_column("script", "Script", true, true),
        list_column("arguments", "Arguments", 256),
        text_column("workingDirectory", "Working directory", true, false),
    ]
}

fn text_column(key: &str, title: &str, path: bool, required: bool) -> SettingColumn {
    SettingColumn {
        key: key.to_owned(),
        title: title.to_owned(),
        control: SettingColumnControl::Text {
            placeholder: None,
            path,
        },
        required,
    }
}

fn list_column(key: &str, title: &str, max_items: u32) -> SettingColumn {
    SettingColumn {
        key: key.to_owned(),
        title: title.to_owned(),
        control: SettingColumnControl::StringList {
            placeholder: None,
            max_items,
        },
        required: false,
    }
}

fn script_row(script: &ScriptEntry) -> BTreeMap<String, SettingValue> {
    BTreeMap::from([
        ("id".to_owned(), text_value(&script.id)),
        ("title".to_owned(), text_value(&script.title)),
        (
            "aliases".to_owned(),
            SettingValue::StringList {
                values: script.aliases.clone(),
            },
        ),
        (
            "interpreter".to_owned(),
            text_value(&script.interpreter.to_string_lossy()),
        ),
        (
            "script".to_owned(),
            text_value(&script.script.to_string_lossy()),
        ),
        (
            "arguments".to_owned(),
            SettingValue::StringList {
                values: script.arguments.clone(),
            },
        ),
        (
            "workingDirectory".to_owned(),
            text_value(
                &script
                    .working_directory
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ),
        ),
    ])
}

fn script_from_row(row: &BTreeMap<String, SettingValue>) -> Result<ScriptEntry, String> {
    let working_directory = row_text(row, "workingDirectory")?;
    Ok(ScriptEntry {
        id: row_text(row, "id")?,
        title: row_text(row, "title")?,
        aliases: row_list(row, "aliases")?,
        interpreter: PathBuf::from(row_text(row, "interpreter")?),
        script: PathBuf::from(row_text(row, "script")?),
        arguments: row_list(row, "arguments")?,
        working_directory: (!working_directory.trim().is_empty())
            .then(|| PathBuf::from(working_directory)),
    })
}

fn text_value(value: &str) -> SettingValue {
    SettingValue::Text {
        value: value.to_owned(),
    }
}

fn row_text(row: &BTreeMap<String, SettingValue>, key: &str) -> Result<String, String> {
    match row.get(key) {
        Some(SettingValue::Text { value }) => Ok(value.clone()),
        _ => Err(format!("script row is missing text field {key}")),
    }
}

fn row_list(row: &BTreeMap<String, SettingValue>, key: &str) -> Result<Vec<String>, String> {
    match row.get(key) {
        Some(SettingValue::StringList { values }) => Ok(values.clone()),
        _ => Err(format!("script row is missing list field {key}")),
    }
}

fn valid_id(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some('a'..='z'))
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !value.ends_with('-')
}

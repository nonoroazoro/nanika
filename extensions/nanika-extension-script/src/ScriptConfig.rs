use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nanika_config::ConfigStore;
use serde::{Deserialize, Serialize};

use crate::{EXTENSION_ID, ScriptEntry};

const FORMAT_VERSION: u32 = 1;
const MAX_SCRIPTS: usize = 5_000;

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
            if !valid_id(&script.id) || !ids.insert(script.id.as_str()) {
                return Err(format!("invalid or duplicate script id: {}", script.id));
            }
            if script.title.trim().is_empty()
                || !script.interpreter.is_absolute()
                || !script.script.is_absolute()
                || script
                    .working_directory
                    .as_ref()
                    .is_some_and(|path| !path.is_absolute())
            {
                return Err(format!("invalid script settings: {}", script.id));
            }
        }
        Ok(())
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

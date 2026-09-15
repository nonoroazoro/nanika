use std::collections::HashSet;
use std::path::Path;

use nanika_protocol::ExtensionConfiguration;
use serde::Deserialize;

use crate::ScriptEntry;

const SCRIPTS_KEY: &str = "script.entries";
const MAX_SCRIPTS: usize = 5_000;
const MAX_TEXT_BYTES: usize = 4_096;

/// Script catalog configuration supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptConfig {
    pub scripts: Vec<ScriptEntry>,
}

impl ScriptConfig {
    pub fn from_configuration(configuration: &ExtensionConfiguration) -> Result<Self, String> {
        let values = configuration
            .values()
            .get(SCRIPTS_KEY)
            .ok_or_else(|| format!("script configuration is missing {SCRIPTS_KEY}"))?
            .as_array()
            .ok_or_else(|| format!("script configuration {SCRIPTS_KEY} must be an array"))?;
        let config = Self {
            scripts: values
                .iter()
                .cloned()
                .map(|value| {
                    serde_json::from_value::<ConfiguredScript>(value)
                        .map(ConfiguredScript::into_entry)
                        .map_err(|error| error.to_string())
                })
                .collect::<Result<_, _>>()?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.scripts.len() > MAX_SCRIPTS {
            return Err(format!(
                "script configuration exceeds the {MAX_SCRIPTS} entry limit"
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
                return Err(format!("invalid script configuration: {}", script.id));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfiguredScript {
    id: String,
    title: String,
    aliases: Vec<String>,
    interpreter: String,
    script: String,
    arguments: Vec<String>,
    working_directory: String,
}

impl ConfiguredScript {
    fn into_entry(self) -> ScriptEntry {
        ScriptEntry {
            id: self.id,
            title: self.title,
            aliases: self.aliases,
            interpreter: self.interpreter.into(),
            script: self.script.into(),
            arguments: self.arguments,
            working_directory: (!self.working_directory.trim().is_empty())
                .then(|| self.working_directory.into()),
        }
    }
}

fn bounded_text(value: &str) -> bool {
    value.len() <= MAX_TEXT_BYTES
}

fn bounded_path(path: &Path) -> bool {
    path.to_string_lossy().len() <= MAX_TEXT_BYTES
}

fn valid_id(id: &str) -> bool {
    let mut characters = id.chars();
    matches!(characters.next(), Some('a'..='z'))
        && characters.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-' | '_')
        })
}

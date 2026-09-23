use std::collections::BTreeSet;
use std::path::PathBuf;

use nanika_protocol::ExtensionConfiguration;

use crate::ApplicationError;

const ROOTS_KEY: &str = "application.roots";
const BUILTIN_ROOT_PREFIX: &str = "application.builtin.";
const MAX_PATHS: usize = 256;
const MAX_PATH_BYTES: usize = 4_096;

/// Application discovery configuration supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationConfig {
    pub roots: Vec<PathBuf>,
    pub exclusions: Vec<PathBuf>,
    pub enabled_builtin_roots: BTreeSet<String>,
}

impl ApplicationConfig {
    pub fn from_configuration(
        configuration: &ExtensionConfiguration,
    ) -> Result<Self, ApplicationError> {
        let values = configuration.values();
        let config = Self {
            roots: path_list(values, ROOTS_KEY)?,
            exclusions: Vec::new(),
            enabled_builtin_roots: enabled_builtin_roots(values)?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ApplicationError> {
        if self.roots.len() > MAX_PATHS || self.exclusions.len() > MAX_PATHS {
            return Err(ApplicationError::Configuration(format!(
                "application discovery lists exceed {MAX_PATHS} paths"
            )));
        }
        for path in self.roots.iter().chain(&self.exclusions) {
            if !path.is_absolute() || path.to_string_lossy().len() > MAX_PATH_BYTES {
                return Err(ApplicationError::Configuration(format!(
                    "application discovery path must be absolute and at most {MAX_PATH_BYTES} bytes: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    pub fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
        crate::platform::standard_roots()
    }
}

fn enabled_builtin_roots(
    values: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Result<BTreeSet<String>, ApplicationError> {
    let mut enabled = BTreeSet::new();
    for (key, value) in values
        .iter()
        .filter(|(key, _)| key.starts_with(BUILTIN_ROOT_PREFIX))
    {
        let value = value.as_bool().ok_or_else(|| {
            ApplicationError::Configuration(format!(
                "application configuration {key} must be a boolean"
            ))
        })?;
        if value {
            enabled.insert(key.clone());
        }
    }
    Ok(enabled)
}

fn path_list(
    values: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<PathBuf>, ApplicationError> {
    values
        .get(key)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ApplicationError::Configuration(format!(
                "application configuration is missing array {key}"
            ))
        })?
        .iter()
        .map(|value| {
            value.as_str().map(PathBuf::from).ok_or_else(|| {
                ApplicationError::Configuration(format!(
                    "application configuration {key} must contain only paths"
                ))
            })
        })
        .collect()
}

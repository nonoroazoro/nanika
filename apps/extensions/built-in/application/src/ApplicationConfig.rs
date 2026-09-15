use std::path::PathBuf;

use nanika_protocol::ExtensionConfiguration;

use crate::ApplicationError;

const ROOTS_KEY: &str = "application.roots";
const EXCLUSIONS_KEY: &str = "application.exclusions";
const MAX_PATHS: usize = 256;
const MAX_PATH_BYTES: usize = 4_096;

/// Application discovery configuration supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationConfig {
    pub roots: Vec<PathBuf>,
    pub exclusions: Vec<PathBuf>,
}

impl ApplicationConfig {
    pub fn from_configuration(
        configuration: &ExtensionConfiguration,
    ) -> Result<Self, ApplicationError> {
        let values = configuration.values();
        let config = Self {
            roots: path_list(values, ROOTS_KEY)?,
            exclusions: path_list(values, EXCLUSIONS_KEY)?,
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

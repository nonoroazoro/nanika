use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ConfigurationProperty, validate_key};

const MAX_CONFIGURATION_BYTES: usize = 1024 * 1024;

/// Static Settings schema declared by one extension package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationContribution {
    pub title: String,
    pub properties: BTreeMap<String, ConfigurationProperty>,
}

impl ConfigurationContribution {
    pub fn validate(&self) -> Result<(), String> {
        validate_text(&self.title, 128, "configuration title")?;
        if self.properties.is_empty() || self.properties.len() > 64 {
            return Err("configuration must contain between 1 and 64 properties".to_owned());
        }
        for (key, property) in &self.properties {
            validate_key(key)?;
            if property.platforms.len() > 2
                || property
                    .platforms
                    .iter()
                    .any(|platform| !matches!(platform.as_str(), "windows" | "macos"))
                || (property.platforms.len() == 2 && property.platforms[0] == property.platforms[1])
            {
                return Err(format!("invalid configuration platforms for {key}"));
            }
            validate_text(&property.title, 128, "configuration property title")?;
            if let Some(description) = &property.description {
                validate_text(description, 512, "configuration property description")?;
            }
            property.schema.validate_definition()?;
            property
                .schema
                .validate_value(&property.default)
                .map_err(|error| {
                    format!("invalid default for configuration property {key}: {error}")
                })?;
        }
        let encoded = serde_json::to_vec(self)
            .map_err(|error| format!("configuration schema cannot be encoded: {error}"))?;
        if encoded.len() > MAX_CONFIGURATION_BYTES {
            return Err(format!(
                "configuration schema exceeds {MAX_CONFIGURATION_BYTES} bytes"
            ));
        }
        Ok(())
    }

    pub fn defaults(&self) -> BTreeMap<String, Value> {
        self.properties
            .iter()
            .map(|(key, property)| (key.clone(), property.default.clone()))
            .collect()
    }

    /// Select presentation properties while retaining the complete value contract separately.
    pub fn for_platform(&self, platform: &str) -> Self {
        let mut visible = self.clone();
        visible.properties.retain(|_, property| {
            property.platforms.is_empty()
                || property
                    .platforms
                    .iter()
                    .any(|candidate| candidate == platform)
        });
        visible
    }

    pub fn validate_values(&self, values: &BTreeMap<String, Value>) -> Result<(), String> {
        for key in values.keys() {
            if !self.properties.contains_key(key) {
                return Err(format!("unknown configuration property: {key}"));
            }
        }
        for (key, property) in &self.properties {
            let value = values
                .get(key)
                .ok_or_else(|| format!("missing configuration property: {key}"))?;
            property
                .schema
                .validate_value(value)
                .map_err(|error| format!("invalid configuration property {key}: {error}"))?;
        }
        Ok(())
    }
}

fn validate_text(value: &str, maximum: usize, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} is empty"))
    } else if value.len() > maximum {
        Err(format!("{label} exceeds {maximum} bytes"))
    } else {
        Ok(())
    }
}

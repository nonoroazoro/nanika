use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ConfigurationProperty, validate_key};

const MAX_CONFIGURATION_BYTES: usize = 1024 * 1024;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationContribution {
    pub title: String,
    pub properties: BTreeMap<String, ConfigurationProperty>,
    #[serde(default)]
    pub actions: Vec<crate::SettingsAction>,
    #[serde(default)]
    pub quick_menu: Vec<crate::ConfigurationMenuGroup>
}

impl ConfigurationContribution {
    pub fn validate(&self) -> Result<(), String> {
        validate_text(&self.title, 128, "configuration title")?;
        if (self.properties.is_empty() && self.actions.is_empty()) || self.properties.len() > 64 {
            return Err(
                "configuration must contain properties or actions, with at most 64 properties"
                    .to_owned()
            );
        }
        for (key, property) in &self.properties {
            validate_key(key)?;
            if property.platforms.len() > 2
                || (property.platforms.len() == 2 && property.platforms[0] == property.platforms[1])
            {
                return Err(format!("invalid configuration platforms for {key}"));
            }
            validate_text(&property.title, 128, "configuration property title")?;
            if let Some(group) = &property.group {
                validate_text(group, 128, "configuration property group")?;
            }
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
        nanika_protocol::validate_actions(
            &self
                .actions
                .iter()
                .map(|item| item.action.clone())
                .collect::<Vec<_>>()
        )?;
        for item in &self.actions {
            validate_text(&item.description, 512, "settings action description")?;
            if let Some(label) = &item.button_label {
                validate_text(label, 128, "settings action button label")?;
            }
            if item.action.allow_default_execution {
                return Err("settings actions require explicit invocation".into());
            }
            if item.action.style == nanika_protocol::ActionStyle::Destructive
                && item.action.confirmation_title.is_none()
            {
                return Err("destructive settings actions require confirmation".into());
            }
        }
        self._validate_quick_menu()?;
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
    pub fn for_platform(&self, platform: nanika_platform::TargetPlatform) -> Self {
        let mut visible = self.clone();
        visible.properties.retain(|_, property| {
            property.platforms.is_empty() || property.platforms.contains(&platform)
        });
        for group in &mut visible.quick_menu {
            group.items.retain(|item| match item {
                crate::ConfigurationMenuItem::Toggle { key } => {
                    visible.properties.contains_key(key)
                }
                crate::ConfigurationMenuItem::Action { .. } => true
            });
        }
        visible.quick_menu.retain(|group| !group.items.is_empty());
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

    pub fn exposes_menu_item(&self, item: &crate::ConfigurationMenuItem) -> bool {
        self.quick_menu
            .iter()
            .any(|group| group.items.contains(item))
    }

    fn _validate_quick_menu(&self) -> Result<(), String> {
        if self.quick_menu.len() > 8 {
            return Err("configuration quick menu exceeds 8 groups".into());
        }
        let mut seen = HashSet::new();
        for group in &self.quick_menu {
            validate_text(&group.title, 128, "configuration menu group title")?;
            if group.items.is_empty() {
                return Err("configuration menu groups must not be empty".into());
            }
            for item in &group.items {
                if !seen.insert(item) || seen.len() > 32 {
                    return Err(
                        "configuration quick menu requires unique entries, at most 32".into()
                    );
                }
                match item {
                    crate::ConfigurationMenuItem::Toggle { key } => {
                        if !self.properties.get(key).is_some_and(|property| {
                            property.schema.value_type == crate::ConfigurationValueType::Boolean
                        }) {
                            return Err(format!(
                                "menu toggle must reference a boolean setting: {key}"
                            ));
                        }
                    }
                    crate::ConfigurationMenuItem::Action { id } => {
                        if !self.actions.iter().any(|item| item.action.id == *id) {
                            return Err(format!("menu action is not declared in Settings: {id}"));
                        }
                    }
                }
            }
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

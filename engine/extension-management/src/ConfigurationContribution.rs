use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ConfigurationProperty, validate_key};

const MAX_CONFIGURATION_BYTES: usize = 1024 * 1024;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationContribution {
    pub title: nanika_protocol::LocalizedText,
    pub properties: BTreeMap<String, ConfigurationProperty>,
    #[serde(default)]
    pub actions: Vec<crate::SettingsAction>,
    #[serde(default)]
    pub quick_menu: Vec<crate::ConfigurationMenuGroup>
}

impl ConfigurationContribution {
    pub fn validate(&self) -> Result<(), String> {
        if (self.properties.is_empty() && self.actions.is_empty()) || self.properties.len() > 64 {
            return Err(
                "configuration must contain properties or actions, with at most 64 properties"
                    .to_owned()
            );
        }
        self._visit_texts(&mut |label, text, maximum| validate_text(text, maximum, label))?;
        self._validate_groups()?;
        for (key, property) in &self.properties {
            validate_key(key)?;
            if property.platforms.len() > 2
                || (property.platforms.len() == 2 && property.platforms[0] == property.platforms[1])
            {
                return Err(format!("invalid configuration platforms for {key}"));
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

    /// Validate every rendered field before admission, then retain bilingual search data.
    pub(crate) fn _prepare_presentation(
        &self,
        resources: Option<&nanika_localization::ResourceSet>
    ) -> Result<BTreeMap<String, Vec<String>>, String> {
        self.validate()?;
        for item in &self.actions {
            nanika_localization::validate_action_presentation(
                std::slice::from_ref(&item.action),
                resources
            )?;
        }
        let locales = [
            nanika_localization::SupportedLocale::English,
            nanika_localization::SupportedLocale::SimplifiedChinese
        ];
        for locale in locales {
            for platform in ["macos", "windows"] {
                self._visit_texts(&mut |field, text, maximum| {
                    let output =
                        _format(text, resources, locale, platform, maximum).map_err(|error| {
                            format!("{field}: {}: {platform}: {error}", locale.as_str())
                        })?;
                    validate_text(&output.into(), maximum, field)
                })?;
            }
        }
        let platform = nanika_platform::target_platform().as_str();
        let mut search = BTreeMap::new();
        for (key, property) in &self.properties {
            let mut values = Vec::new();
            for locale in locales {
                for text in [
                    &property.title,
                    property
                        .group
                        .as_ref()
                        .map(|group| &group.title)
                        .unwrap_or(&self.title)
                ] {
                    let value = _format(text, resources, locale, platform, 128)?;
                    if !values.contains(&value) {
                        values.push(value);
                    }
                }
            }
            search.insert(format!("field:{key}"), values);
        }
        for item in &self.actions {
            let mut values = Vec::new();
            for locale in locales {
                for text in std::iter::once(&item.action.title)
                    .chain(item.action.group.as_ref().map(|group| &group.title))
                {
                    let value = _format(text, resources, locale, platform, 128)?;
                    if !values.contains(&value) {
                        values.push(value);
                    }
                }
            }
            search.insert(format!("action:{}", item.action.id), values);
        }
        Ok(search)
    }

    /// Visit presentation only; schema keys, defaults, and stored values never enter Fluent.
    fn _visit_texts(
        &self,
        visitor: &mut impl FnMut(&str, &nanika_protocol::LocalizedText, usize) -> Result<(), String>
    ) -> Result<(), String> {
        visitor("title", &self.title, 128)?;
        for (key, property) in &self.properties {
            visitor(&format!("properties.{key}.title"), &property.title, 128)?;
            if let Some(group) = &property.group {
                visitor(&format!("properties.{key}.group.title"), &group.title, 128)?;
            }
            if let Some(description) = &property.description {
                visitor(&format!("properties.{key}.description"), description, 512)?;
            }
        }
        for item in &self.actions {
            visitor(
                &format!("actions.{}.description", item.action.id),
                &item.description,
                512
            )?;
            if let Some(label) = &item.button_label {
                visitor(
                    &format!("actions.{}.buttonLabel", item.action.id),
                    label,
                    128
                )?;
            }
        }
        for group in &self.quick_menu {
            visitor(&format!("quickMenu.{}.title", group.id), &group.title, 128)?;
        }
        Ok(())
    }

    fn _validate_groups(&self) -> Result<(), String> {
        let mut definitions = self
            .properties
            .values()
            .filter_map(|property| property.group.clone())
            .chain(
                self.actions
                    .iter()
                    .filter_map(|item| item.action.group.clone())
            )
            .collect::<Vec<_>>();
        let mut menus = HashSet::new();
        for group in &self.quick_menu {
            if !menus.insert(&group.id) {
                return Err("duplicate configuration menu group id".into());
            }
            definitions.push(nanika_protocol::PresentationGroup {
                id: group.id.clone(),
                title: group.title.clone()
            });
        }
        nanika_protocol::PresentationGroup::validate_definitions(&definitions)
    }

    fn _validate_quick_menu(&self) -> Result<(), String> {
        if self.quick_menu.len() > 8 {
            return Err("configuration quick menu exceeds 8 groups".into());
        }
        let mut seen = HashSet::new();
        for group in &self.quick_menu {
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

fn validate_text(
    value: &nanika_protocol::LocalizedText,
    maximum: usize,
    label: &str
) -> Result<(), String> {
    let value = match value {
        nanika_protocol::LocalizedText::Literal(value) => value,
        nanika_protocol::LocalizedText::Message(reference) => {
            return reference
                .validate()
                .map_err(|error| format!("{label}: {error}"));
        }
    };
    if value.trim().is_empty() {
        Err(format!("{label} is empty"))
    } else if value.len() > maximum {
        Err(format!("{label} exceeds {maximum} bytes"))
    } else {
        Ok(())
    }
}

fn _format(
    text: &nanika_protocol::LocalizedText,
    resources: Option<&nanika_localization::ResourceSet>,
    locale: nanika_localization::SupportedLocale,
    platform: &str,
    maximum: usize
) -> Result<String, String> {
    match text {
        nanika_protocol::LocalizedText::Literal(value) => Ok(value.clone()),
        nanika_protocol::LocalizedText::Message(_) => resources
            .ok_or("message reference requires localization resources")?
            .format(locale, text, platform, maximum, false)
    }
}

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_PROPERTIES: usize = 64;
const MAX_SCHEMA_DEPTH: usize = 4;
const MAX_STRING_BYTES: u64 = 4_096;
const MAX_ARRAY_ITEMS: u64 = 5_000;

/// The bounded JSON Schema subset supported by Nanika Settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationSchema {
    #[serde(rename = "type")]
    pub value_type: ConfigurationValueType,
    #[serde(default)]
    pub format: Option<ConfigurationStringFormat>,
    #[serde(default)]
    pub minimum: Option<i64>,
    #[serde(default)]
    pub maximum: Option<i64>,
    #[serde(default)]
    pub multiple_of: Option<u64>,
    #[serde(default)]
    pub max_length: Option<u64>,
    #[serde(default)]
    pub max_items: Option<u64>,
    #[serde(default)]
    pub items: Option<Box<ConfigurationSchema>>,
    #[serde(default)]
    pub properties: BTreeMap<String, ConfigurationSchema>,
    #[serde(default)]
    pub required: Vec<String>,
}

impl ConfigurationSchema {
    pub(crate) fn validate_definition(&self) -> Result<(), String> {
        self.validate_definition_at_depth(0)
    }

    pub fn validate_value(&self, value: &Value) -> Result<(), String> {
        self.validate_value_at_path(value, "configuration value")
    }

    fn validate_definition_at_depth(&self, depth: usize) -> Result<(), String> {
        if depth > MAX_SCHEMA_DEPTH {
            return Err(format!(
                "configuration schema exceeds depth {MAX_SCHEMA_DEPTH}"
            ));
        }
        match self.value_type {
            ConfigurationValueType::Boolean => self.require_no_shape_constraints(),
            ConfigurationValueType::Integer => {
                self.require_no_collection_constraints()?;
                if self.format.is_some() || self.max_length.is_some() {
                    return Err("integer configuration has string constraints".to_owned());
                }
                let minimum = self
                    .minimum
                    .ok_or_else(|| "integer configuration is missing minimum".to_owned())?;
                let maximum = self
                    .maximum
                    .ok_or_else(|| "integer configuration is missing maximum".to_owned())?;
                let multiple_of = self
                    .multiple_of
                    .ok_or_else(|| "integer configuration is missing multipleOf".to_owned())?;
                if minimum > maximum || multiple_of == 0 {
                    return Err("integer configuration has invalid bounds".to_owned());
                }
                Ok(())
            }
            ConfigurationValueType::String => {
                self.require_no_number_constraints()?;
                self.require_no_collection_constraints()?;
                if self.max_length.unwrap_or(MAX_STRING_BYTES) > MAX_STRING_BYTES {
                    return Err(format!(
                        "string configuration exceeds the {MAX_STRING_BYTES} byte limit"
                    ));
                }
                Ok(())
            }
            ConfigurationValueType::Array => {
                self.require_no_number_constraints()?;
                if self.format.is_some()
                    || self.max_length.is_some()
                    || !self.properties.is_empty()
                    || !self.required.is_empty()
                {
                    return Err("array configuration has unrelated constraints".to_owned());
                }
                let max_items = self
                    .max_items
                    .ok_or_else(|| "array configuration is missing maxItems".to_owned())?;
                if max_items == 0 || max_items > MAX_ARRAY_ITEMS {
                    return Err(format!(
                        "array configuration exceeds the {MAX_ARRAY_ITEMS} item limit"
                    ));
                }
                self.items
                    .as_deref()
                    .ok_or_else(|| "array configuration is missing items".to_owned())?
                    .validate_definition_at_depth(depth + 1)
            }
            ConfigurationValueType::Object => {
                self.require_no_number_constraints()?;
                if self.format.is_some()
                    || self.max_length.is_some()
                    || self.max_items.is_some()
                    || self.items.is_some()
                {
                    return Err("object configuration has unrelated constraints".to_owned());
                }
                if self.properties.is_empty() || self.properties.len() > MAX_PROPERTIES {
                    return Err(format!(
                        "object configuration must contain between 1 and {MAX_PROPERTIES} properties"
                    ));
                }
                let mut required = HashSet::with_capacity(self.required.len());
                for key in &self.required {
                    if !self.properties.contains_key(key) || !required.insert(key) {
                        return Err(format!(
                            "object configuration has invalid required property: {key}"
                        ));
                    }
                }
                for (key, schema) in &self.properties {
                    validate_key(key)?;
                    schema.validate_definition_at_depth(depth + 1)?;
                }
                Ok(())
            }
        }
    }

    fn validate_value_at_path(&self, value: &Value, path: &str) -> Result<(), String> {
        match self.value_type {
            ConfigurationValueType::Boolean if value.is_boolean() => Ok(()),
            ConfigurationValueType::Integer => {
                let value = value
                    .as_i64()
                    .ok_or_else(|| format!("{path} must be an integer"))?;
                let minimum = self
                    .minimum
                    .ok_or_else(|| format!("{path} schema is missing minimum"))?;
                let maximum = self
                    .maximum
                    .ok_or_else(|| format!("{path} schema is missing maximum"))?;
                let multiple_of = self
                    .multiple_of
                    .ok_or_else(|| format!("{path} schema is missing multipleOf"))?;
                if value < minimum || value > maximum {
                    return Err(format!("{path} is outside its bounds"));
                }
                if i128::from(value) % i128::from(multiple_of) != 0 {
                    return Err(format!("{path} does not match multipleOf"));
                }
                Ok(())
            }
            ConfigurationValueType::String => {
                let value = value
                    .as_str()
                    .ok_or_else(|| format!("{path} must be a string"))?;
                let maximum = self.max_length.unwrap_or(MAX_STRING_BYTES);
                if value.len() as u64 > maximum {
                    return Err(format!("{path} exceeds {maximum} bytes"));
                }
                Ok(())
            }
            ConfigurationValueType::Array => {
                let values = value
                    .as_array()
                    .ok_or_else(|| format!("{path} must be an array"))?;
                let maximum = self
                    .max_items
                    .ok_or_else(|| format!("{path} schema is missing maxItems"))?;
                if values.len() as u64 > maximum {
                    return Err(format!("{path} exceeds {maximum} items"));
                }
                let items = self
                    .items
                    .as_deref()
                    .ok_or_else(|| format!("{path} schema is missing items"))?;
                for (index, value) in values.iter().enumerate() {
                    items.validate_value_at_path(value, &format!("{path}[{index}]"))?;
                }
                Ok(())
            }
            ConfigurationValueType::Object => {
                let values = value
                    .as_object()
                    .ok_or_else(|| format!("{path} must be an object"))?;
                for key in values.keys() {
                    if !self.properties.contains_key(key) {
                        return Err(format!("{path} contains unknown property {key}"));
                    }
                }
                for key in &self.required {
                    if !values.contains_key(key) {
                        return Err(format!("{path} is missing required property {key}"));
                    }
                }
                for (key, value) in values {
                    let schema = self
                        .properties
                        .get(key)
                        .ok_or_else(|| format!("{path} contains unknown property {key}"))?;
                    schema.validate_value_at_path(value, &format!("{path}.{key}"))?;
                }
                Ok(())
            }
            ConfigurationValueType::Boolean => Err(format!("{path} must be a boolean")),
        }
    }

    fn require_no_shape_constraints(&self) -> Result<(), String> {
        self.require_no_number_constraints()?;
        self.require_no_collection_constraints()?;
        if self.format.is_some() || self.max_length.is_some() {
            return Err("boolean configuration has unrelated constraints".to_owned());
        }
        Ok(())
    }

    fn require_no_number_constraints(&self) -> Result<(), String> {
        if self.minimum.is_some() || self.maximum.is_some() || self.multiple_of.is_some() {
            Err("configuration has unrelated number constraints".to_owned())
        } else {
            Ok(())
        }
    }

    fn require_no_collection_constraints(&self) -> Result<(), String> {
        if self.max_items.is_some()
            || self.items.is_some()
            || !self.properties.is_empty()
            || !self.required.is_empty()
        {
            Err("configuration has unrelated collection constraints".to_owned())
        } else {
            Ok(())
        }
    }
}

/// JSON value categories supported by the declarative Settings renderer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConfigurationValueType {
    Boolean,
    Integer,
    String,
    Array,
    Object,
}

/// Presentation hint for string values. It does not change JSON validation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConfigurationStringFormat {
    Path,
}

pub(crate) fn validate_key(value: &str) -> Result<(), String> {
    let mut characters = value.chars();
    if value.len() > 128
        || !matches!(characters.next(), Some('a'..='z'))
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        Err(format!("invalid configuration key: {value}"))
    } else {
        Ok(())
    }
}

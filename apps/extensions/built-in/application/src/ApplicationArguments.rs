use serde::{Deserialize, Serialize};

use crate::ApplicationError;

/// Persisted application arguments with an explicit platform representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApplicationArguments {
    Structured { values: Vec<String> },
    WindowsRaw { value: String },
}

impl ApplicationArguments {
    pub const fn empty() -> Self {
        Self::Structured { values: Vec::new() }
    }

    pub fn from_windows_raw(value: Option<String>) -> Self {
        match value.filter(|value| !value.trim().is_empty()) {
            Some(value) => Self::WindowsRaw { value },
            None => Self::empty(),
        }
    }

    pub fn to_json(&self) -> Result<String, ApplicationError> {
        Ok(serde_json::to_string(self)?)
    }
}

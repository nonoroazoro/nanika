use crate::LocalizedText;
use serde::{Deserialize, Serialize};

/// Metadata keeps measured values typed until the frontend presentation boundary.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
pub enum ViewMetadataValue {
    Text(LocalizedText),
    Bytes { bytes: u64 }
}

impl ViewMetadataValue {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if let Self::Bytes { bytes } = self
            && *bytes > crate::MAX_MESSAGE_INTEGER as u64
        {
            return Err("detail metadata byte count exceeds the JSON-safe integer range".into());
        }
        Ok(())
    }
}

impl From<LocalizedText> for ViewMetadataValue {
    fn from(value: LocalizedText) -> Self {
        Self::Text(value)
    }
}

impl From<String> for ViewMetadataValue {
    fn from(value: String) -> Self {
        Self::Text(value.into())
    }
}

impl From<&str> for ViewMetadataValue {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}

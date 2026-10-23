use serde::{Deserialize, Serialize};

/// Counts are JSON-safe integers; locale-sensitive decimals remain typed domain data.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum MessageArgument {
    Text(String),
    Integer(i64)
}

impl From<&str> for MessageArgument {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for MessageArgument {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<i64> for MessageArgument {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

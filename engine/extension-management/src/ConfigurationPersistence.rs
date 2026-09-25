use serde::{Deserialize, Serialize};

/// Controls when a requested value becomes durable relative to live application.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConfigurationPersistence {
    BeforeApply,
    AfterApply,
}

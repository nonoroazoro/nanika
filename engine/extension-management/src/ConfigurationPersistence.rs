use serde::{Deserialize, Serialize};

/// Controls when a requested value becomes durable relative to live application.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConfigurationPersistence {
    BeforeApply,
    AfterApply,
}

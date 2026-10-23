use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ConfigurationPersistence, ConfigurationSchema};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationProperty {
    pub title: nanika_protocol::LocalizedText,
    /// Stable presentation group. Omission uses the contribution title with an implicit identity.
    #[serde(default)]
    pub group: Option<nanika_protocol::PresentationGroup>,
    #[serde(default)]
    pub description: Option<nanika_protocol::LocalizedText>,
    pub default: Value,
    pub persistence: ConfigurationPersistence,
    /// Empty means the property is shown on every supported platform.
    #[serde(default)]
    pub platforms: Vec<nanika_platform::TargetPlatform>,
    #[serde(default)]
    pub order: u16,
    #[serde(flatten)]
    pub schema: ConfigurationSchema
}

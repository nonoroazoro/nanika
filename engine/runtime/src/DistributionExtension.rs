use serde::Deserialize;

use nanika_extension_package::{ExtensionContributions, ExtensionProtocol};

/// One built-in extension shipped by the current Nanika distribution.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DistributionExtension {
    pub id: String,
    pub binary_name: String,
    pub runtime: ExtensionProtocol,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub contributions: ExtensionContributions,
}

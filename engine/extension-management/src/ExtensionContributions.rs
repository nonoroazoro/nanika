use serde::{Deserialize, Serialize};

use crate::CommandContribution;

/// Static host integration declared by an extension package.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionContributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub root_search: bool,
}

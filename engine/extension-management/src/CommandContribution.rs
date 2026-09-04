use serde::{Deserialize, Serialize};

use crate::CommandMode;

/// One statically discoverable command contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    pub description: String,
    pub mode: CommandMode,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

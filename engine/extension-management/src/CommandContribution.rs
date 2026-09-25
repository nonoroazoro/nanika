use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandContribution {
    pub command: String,
    /// Static commands use the same action contract as runtime search and views.
    pub action: nanika_protocol::Action,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

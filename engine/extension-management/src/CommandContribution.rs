use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandContribution {
    pub command: String,
    /// Static commands use the same action contract as runtime search and views.
    pub action: nanika_protocol::Action,
    pub title: nanika_protocol::LocalizedText,
    pub description: nanika_protocol::LocalizedText,
    #[serde(default)]
    pub category: Option<nanika_protocol::LocalizedText>,
    #[serde(default)]
    pub keywords: Vec<nanika_protocol::LocalizedText>,
    #[serde(default)]
    pub icon: Option<String>
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViewContribution {
    pub id: String,
    pub title: nanika_protocol::LocalizedText,
    pub description: nanika_protocol::LocalizedText,
    #[serde(default)]
    pub category: Option<nanika_protocol::LocalizedText>,
    #[serde(default)]
    pub keywords: Vec<nanika_protocol::LocalizedText>,
    #[serde(default)]
    pub icon: Option<String>
}

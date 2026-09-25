use serde::{Deserialize, Serialize};

use crate::{
    CommandContribution, ConfigurationContribution, RootSearchContribution, ViewContribution,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionContributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub views: Vec<ViewContribution>,
    #[serde(default)]
    pub configuration: Option<ConfigurationContribution>,
    #[serde(default)]
    pub root_search: Option<RootSearchContribution>,
}

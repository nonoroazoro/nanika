use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One complete, host-validated extension configuration snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ExtensionConfiguration {
    values: BTreeMap<String, Value>,
}

impl ExtensionConfiguration {
    pub fn new(values: BTreeMap<String, Value>) -> Self {
        Self { values }
    }

    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }

    pub fn into_values(self) -> BTreeMap<String, Value> {
        self.values
    }
}

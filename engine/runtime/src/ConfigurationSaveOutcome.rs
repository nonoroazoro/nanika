use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A missing effective snapshot means unconfirmed application, not rollback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationSaveOutcome {
    pub values: BTreeMap<String, Value>,
    pub saved: BTreeMap<String, Value>,
    pub effective: Option<BTreeMap<String, Value>>,
    pub error: Option<String>,
}

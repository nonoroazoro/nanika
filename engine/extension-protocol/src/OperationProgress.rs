use serde::{Deserialize, Serialize};

/// Task-owned work units, never an estimated duration. A null total is indeterminate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationProgress {
    pub label: String,
    pub completed: u32,
    pub total: Option<u32>,
}

impl OperationProgress {
    pub fn validate(&self) -> Result<(), String> {
        if self.label.trim().is_empty() || self.label.len() > 128 {
            return Err("progress label must contain 1 to 128 bytes".to_owned());
        }
        match self.total {
            Some(total) if total == 0 || self.completed > total => {
                Err("invalid progress work units".to_owned())
            }
            None if self.completed != 0 => {
                Err("indeterminate progress must not report completed units".to_owned())
            }
            _ => Ok(()),
        }
    }
}

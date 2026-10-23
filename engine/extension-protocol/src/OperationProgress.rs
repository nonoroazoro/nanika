use serde::{Deserialize, Serialize};

/// Task-owned work units, never an estimated duration. A null total is indeterminate.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationProgress {
    pub label: crate::LocalizedText,
    pub completed: u32,
    pub total: Option<u32>
}

impl OperationProgress {
    pub fn validate(&self) -> Result<(), String> {
        match &self.label {
            crate::LocalizedText::Literal(label) => {
                if label.trim().is_empty()
                    || label.len() > 128
                    || label.chars().any(|character| {
                        character.is_control() && !matches!(character, '\n' | '\r' | '\t')
                    })
                {
                    return Err("progress label must contain 1 to 128 bytes".to_owned());
                }
            }
            crate::LocalizedText::Message(reference) => reference.validate()?
        }
        match self.total {
            Some(total) if total == 0 || self.completed > total => {
                Err("invalid progress work units".to_owned())
            }
            None if self.completed != 0 => {
                Err("indeterminate progress must not report completed units".to_owned())
            }
            _ => Ok(())
        }
    }
}

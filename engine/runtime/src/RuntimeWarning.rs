use crate::OperationError;

/// A capability failure with its owner and admitted presentation retained separately.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeWarning {
    pub extension_id: Option<String>,
    pub error: OperationError
}

impl RuntimeWarning {
    pub fn new(extension_id: Option<String>, error: OperationError) -> Self {
        Self {
            extension_id,
            error
        }
    }
}

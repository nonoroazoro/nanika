use serde::{Deserialize, Serialize};

/// File identity for display only; the path never authorizes filesystem access.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewFile {
    pub name: String,
    pub path: String,
    pub icon: Option<crate::IconReference>
}

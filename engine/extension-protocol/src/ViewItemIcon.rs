use serde::{Deserialize, Serialize};

/// Semantic artwork or an opaque icon in the owning extension's cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ViewItemIcon {
    Text,
    Files,
    Image,
    Native(crate::IconReference),
}

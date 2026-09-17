use serde::{Deserialize, Serialize};

/// A bounded host-rendered icon for a static contribution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContributionIcon {
    Clipboard,
}

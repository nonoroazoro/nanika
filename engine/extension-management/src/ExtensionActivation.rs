use serde::{Deserialize, Serialize};

/// Startup preserves background listeners; on-demand is explicit and static-only.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionActivation {
    #[default]
    Startup,
    OnDemand,
}

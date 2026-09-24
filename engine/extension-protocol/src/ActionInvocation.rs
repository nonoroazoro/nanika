use serde::{Deserialize, Serialize};

/// Host-reported intent, validated against the current action before dispatch.
/// Confirmation is a trusted host UI acknowledgement, not an extension permission.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionInvocation {
    /// Activate a result or a view's primary action without choosing an action.
    Default,
    /// Choose a specific action in a menu or status bar.
    Explicit,
    /// Choose an action and accept its confirmation in the host UI.
    Confirmed,
}

use serde::{Deserialize, Serialize};

use crate::{ActionInvocation, ActionStyle};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub id: String,
    pub title: String,
    /// Replacement label that requires a second click before invoking a destructive action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_title: Option<String>,
    pub style: ActionStyle,
    pub enabled: bool,
    /// Permit direct activation without choosing an action. Incompatible with confirmation.
    pub allow_default_execution: bool,
    pub group: Option<String>,
}

impl Action {
    /// Authorize against current metadata, including after queued input changes a view.
    pub fn allows_invocation(&self, invocation: ActionInvocation) -> bool {
        self.enabled
            && match invocation {
                ActionInvocation::Default => {
                    self.allow_default_execution && self.confirmation_title.is_none()
                }
                ActionInvocation::Explicit => self.confirmation_title.is_none(),
                ActionInvocation::Confirmed => true,
            }
    }

    /// Create an ordinary primary action that permits direct activation.
    pub fn primary(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            confirmation_title: None,
            style: ActionStyle::Primary,
            enabled: true,
            allow_default_execution: true,
            group: None,
        }
    }
}

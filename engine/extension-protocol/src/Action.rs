use serde::{Deserialize, Serialize};

use crate::{ActionInvocation, ActionStyle};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub id: String,
    pub title: crate::LocalizedText,
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<crate::ActionIcon>,
    /// Question displayed in a confirmation dialog before explicit invocation.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_title: Option<crate::LocalizedText>,
    /// Concrete consequences of accepting the confirmation.
    #[cfg_attr(feature = "typescript", ts(optional))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_description: Option<crate::LocalizedText>,
    pub style: ActionStyle,
    pub enabled: bool,
    /// Permit direct activation without choosing an action. Incompatible with confirmation.
    pub allow_default_execution: bool,
    /// Stable grouping shared by menus and Settings, independently of translated titles.
    pub group: Option<crate::PresentationGroup>
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
                ActionInvocation::Confirmed => true
            }
    }

    /// Create an ordinary primary action that permits direct activation.
    pub fn primary(id: impl Into<String>, title: impl Into<crate::LocalizedText>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            confirmation_description: None,
            confirmation_title: None,
            style: ActionStyle::Primary,
            enabled: true,
            allow_default_execution: true,
            group: None
        }
    }

    /// Visit presentation fields with their existing character limits, excluding invocation data.
    pub fn visit_texts(
        &self,
        visitor: &mut impl FnMut(&str, &crate::LocalizedText, usize) -> Result<(), String>
    ) -> Result<(), String> {
        visitor("action title", &self.title, 128)?;
        if let Some(title) = &self.confirmation_title {
            visitor("action confirmation title", title, 128)?;
        }
        if let Some(description) = &self.confirmation_description {
            visitor("action confirmation description", description, 512)?;
        }
        if let Some(group) = &self.group {
            visitor("action group title", &group.title, 128)?;
        }
        Ok(())
    }
}

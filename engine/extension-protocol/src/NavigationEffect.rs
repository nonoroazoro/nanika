use serde::{Deserialize, Serialize};

use crate::View;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NavigationEffect {
    None,
    /// Hide the launcher while retaining the navigation stack and extension views.
    Dismiss,
    Pop,
    Push {
        view_id: String,
        revision: u64,
        view: Box<View>,
    },
}

impl NavigationEffect {
    pub fn validate(&self) -> Result<(), String> {
        let Self::Push {
            view_id,
            revision,
            view,
        } = self
        else {
            return Ok(());
        };
        if view_id.is_empty()
            || view_id.len() > 128
            || view_id
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        {
            return Err("extension view id is invalid".to_owned());
        }
        if *revision == 0 {
            return Err("extension view revision must be positive".to_owned());
        }
        view.validate()
    }
}

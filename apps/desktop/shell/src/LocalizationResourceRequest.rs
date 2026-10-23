use nanika_host::{OperationError, OperationErrorCode as Code};
use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(
    tag = "surface",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum LocalizationResourceRequest {
    Settings {
        session_id: u64,
        scope: nanika_host::ExtensionResourceScope
    },
    Launcher {
        session_id: u64,
        instance_id: u64,
        scope: nanika_host::ExtensionResourceScope
    }
}

impl LocalizationResourceRequest {
    pub(crate) fn authorize_session(
        &self,
        launcher: Option<&crate::SearchSession>,
        settings: &crate::SettingsApplications
    ) -> Result<(), OperationError> {
        match self {
            Self::Settings { session_id, .. } => {
                if *session_id == 0
                    || *session_id != settings.session_id
                    || settings.updates.is_none()
                {
                    return Err(OperationError::new(Code::SessionUnavailable));
                }
                Ok(())
            }
            Self::Launcher { session_id, .. } => launcher
                .ok_or_else(|| OperationError::new(Code::SessionUnavailable))?
                .authorize(*session_id)
        }
    }
}

#[cfg(test)]
#[path = "../tests/LocalizationResourceRequest.rs"]
mod tests;

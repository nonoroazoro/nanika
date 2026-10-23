use nanika_host::OperationError;

/// Fixed recovery stages preserve every failure without recursive error groups.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub(crate) struct HostSettingsFailure {
    pub(crate) primary: OperationError,
    pub(crate) rollback: Option<Box<OperationError>>,
    pub(crate) cleanup: Option<Box<OperationError>>,
    pub(crate) confirmation: Option<Box<OperationError>>
}

impl HostSettingsFailure {
    pub(crate) fn confirmation_failed(primary: Option<&Self>, detail: String) -> Self {
        let error = OperationError::with_detail(
            nanika_host::OperationErrorCode::PreferencesConfirmationFailed,
            detail
        );
        match primary {
            Some(primary) => Self {
                confirmation: Some(Box::new(error)),
                ..primary.clone()
            },
            None => error.into()
        }
    }
}

impl From<OperationError> for HostSettingsFailure {
    fn from(primary: OperationError) -> Self {
        Self {
            primary,
            rollback: None,
            cleanup: None,
            confirmation: None
        }
    }
}

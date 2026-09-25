use serde::Serialize;

/// Separates durable preferences from confirmed platform state.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsWriteResult<T> {
    pub(crate) values: T,
    pub(crate) saved: T,
    pub(crate) effective: Option<T>,
    pub(crate) error: Option<String>,
}

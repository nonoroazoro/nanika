use serde::Serialize;

/// Separates durable preferences from confirmed platform state.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsWriteResult<T> {
    pub(crate) values: T,
    pub(crate) saved: T,
    pub(crate) effective: Option<T>,
    pub(crate) error: Option<String>,
}

use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ViewOperation {
    Event { event: nanika_protocol::ViewEvent },
    Back,
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ViewEventRequest {
    pub(crate) session_id: u64,
    pub(crate) route_id: u64,
    pub(crate) revision: u64,
    pub(crate) operation: ViewOperation,
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ViewOperation {
    Event { event: nanika_protocol::ViewEvent },
    Back,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ViewEventRequest {
    pub(crate) session_id: u64,
    pub(crate) route_id: u64,
    pub(crate) revision: u64,
    pub(crate) operation: ViewOperation,
}

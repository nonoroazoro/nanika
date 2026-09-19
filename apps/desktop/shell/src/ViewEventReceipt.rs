/// The RPC reports completion; the Channel remains the sole source of UI state.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ViewEventReceipt {
    pub(crate) view_revision: u64,
    pub(crate) navigation_revision: u64,
}

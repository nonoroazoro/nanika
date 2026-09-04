use crate::ExtensionViewUpdatePayload;

/// Completed extension view interaction delivered to the UI thread.
#[derive(Debug, Clone)]
pub(crate) struct ExtensionViewUpdate {
    pub(crate) request_id: u64,
    pub(crate) extension_id: String,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) result: Result<ExtensionViewUpdatePayload, String>,
}

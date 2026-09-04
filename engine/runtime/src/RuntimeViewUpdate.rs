use crate::RuntimeViewCompletion;

/// Completed extension view interaction delivered across the runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeViewUpdate {
    pub request_id: u64,
    pub extension_id: String,
    pub generation: u64,
    pub view_id: String,
    pub result: Result<RuntimeViewCompletion, String>,
}

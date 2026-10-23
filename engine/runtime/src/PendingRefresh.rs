use std::sync::Arc;

pub(crate) struct PendingRefresh {
    pub(crate) id: String,
    pub(crate) generation: u64,
    pub(crate) completion: crate::RefreshCompletion,
    pub(crate) resources: Option<Arc<nanika_localization::ResourceSet>>
}

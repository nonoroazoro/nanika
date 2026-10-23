use std::sync::Arc;

pub(crate) struct PendingTextRead {
    pub(crate) id: String,
    pub(crate) generation: u64,
    pub(crate) read: nanika_protocol::TextRead,
    pub(crate) completion: crate::TextReadCompletion,
    pub(crate) resources: Option<Arc<nanika_localization::ResourceSet>>
}

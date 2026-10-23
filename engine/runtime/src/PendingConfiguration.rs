use std::sync::Arc;

pub(crate) struct PendingConfiguration {
    pub(crate) id: String,
    pub(crate) progress: crate::ConfigurationProgressHandler,
    pub(crate) completion: crate::ConfigurationCompletion,
    pub(crate) resources: Option<Arc<nanika_localization::ResourceSet>>
}

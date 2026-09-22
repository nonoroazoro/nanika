use crate::ExtensionViewRequestKind;

#[derive(Debug, Clone)]
pub(crate) struct ExtensionViewRequest {
    pub(crate) completion: std::sync::mpsc::Sender<Result<crate::RuntimeViewCompletion, String>>,
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) revision: u64,
    pub(crate) kind: ExtensionViewRequestKind,
}

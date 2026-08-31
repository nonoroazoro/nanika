use crate::ExtensionViewRequestKind;

/// One serialized interaction with an extension view session.
#[derive(Debug, Clone)]
pub(crate) struct ExtensionViewRequest {
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) revision: u64,
    pub(crate) kind: ExtensionViewRequestKind,
}

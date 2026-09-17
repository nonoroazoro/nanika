/// Coalescible notification that an extension-owned view should be requested again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeViewInvalidation {
    pub extension_id: String,
    pub view_id: String,
}

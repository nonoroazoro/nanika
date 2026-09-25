/// Coalescible notification that an extension-owned view should be requested again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeViewInvalidation {
    pub instance_id: u64,
    pub extension_id: String,
    pub view_id: String,
}

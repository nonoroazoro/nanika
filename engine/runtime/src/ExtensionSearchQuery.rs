#[derive(Debug, Clone)]
pub(crate) struct ExtensionSearchQuery {
    pub(crate) generation: u64,
    pub(crate) query: String,
}

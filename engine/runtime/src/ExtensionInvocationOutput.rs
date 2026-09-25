#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionInvocationOutput {
    pub(crate) instance_id: u64,
    pub(crate) invocation_id: u64,
    pub(crate) extension_id: String,
    pub(crate) generation: u64,
    pub(crate) text: String,
}

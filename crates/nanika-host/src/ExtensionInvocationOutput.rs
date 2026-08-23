/// Protocol-neutral output produced by an extension action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionInvocationOutput {
    pub(crate) invocation_id: u64,
    pub(crate) extension_id: String,
    pub(crate) generation: u64,
    pub(crate) text: String,
}

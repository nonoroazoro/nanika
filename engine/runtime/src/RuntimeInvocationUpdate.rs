use crate::RuntimeInvocationCompletion;

/// Completed extension invocation delivered across the runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInvocationUpdate {
    pub invocation_id: u64,
    pub extension_id: String,
    pub generation: u64,
    pub entry_id: String,
    pub action_id: String,
    pub query_context: String,
    pub result: Result<RuntimeInvocationCompletion, String>,
}

/// Latest bounded output produced by a streaming extension invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOutputUpdate {
    pub invocation_id: u64,
    pub extension_id: String,
    pub generation: u64,
    pub text: String,
}

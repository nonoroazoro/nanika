#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOutputUpdate {
    pub instance_id: u64,
    pub invocation_id: u64,
    pub extension_id: String,
    pub generation: u64,
    pub text: String,
}

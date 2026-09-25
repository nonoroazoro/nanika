/// Reports correlated progress without changing transaction completion semantics.
pub type ConfigurationProgressHandler =
    std::sync::Arc<dyn Fn(nanika_protocol::OperationProgress) + Send + Sync>;

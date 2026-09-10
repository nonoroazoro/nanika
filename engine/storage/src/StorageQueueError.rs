/// Failure to enqueue a storage command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageQueueError {
    Closed,
    InvalidExtensionId,
    Operation(String),
}

impl std::fmt::Display for StorageQueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => formatter.write_str("storage owner is closed"),
            Self::InvalidExtensionId => formatter.write_str("extension id is invalid"),
            Self::Operation(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for StorageQueueError {}

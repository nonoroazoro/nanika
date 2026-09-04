/// Failure to enqueue a storage command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageQueueError {
    Full,
    Closed,
    InvalidExtensionId,
}

impl std::fmt::Display for StorageQueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => formatter.write_str("storage queue is full"),
            Self::Closed => formatter.write_str("storage owner is closed"),
            Self::InvalidExtensionId => formatter.write_str("extension id is invalid"),
        }
    }
}

impl std::error::Error for StorageQueueError {}

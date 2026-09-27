#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchQueueError {
    Closed,
    Retired,
    AlreadyRegistered,
    QueryTooLong,
}

impl std::fmt::Display for SearchQueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retired => formatter.write_str("extension search authority is retired"),
            Self::AlreadyRegistered => {
                formatter.write_str("extension already has a search authority")
            }
            Self::Closed => formatter.write_str("search owner is closed"),
            Self::QueryTooLong => formatter.write_str("search query exceeds the character limit"),
        }
    }
}

impl std::error::Error for SearchQueueError {}

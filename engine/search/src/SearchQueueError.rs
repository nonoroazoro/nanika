/// Search owner boundary failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchQueueError {
    Closed,
    QueryTooLong,
}

impl std::fmt::Display for SearchQueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => formatter.write_str("search owner is closed"),
            Self::QueryTooLong => formatter.write_str("search query exceeds the character limit"),
        }
    }
}

impl std::error::Error for SearchQueueError {}

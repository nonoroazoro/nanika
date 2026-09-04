use std::fmt::{Display, Formatter};

/// Error at the application extension boundary.
#[derive(Debug)]
pub enum ApplicationError {
    Configuration(String),
    Database(rusqlite::Error),
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl ApplicationError {
    pub(crate) fn is_corrupt_database(&self) -> bool {
        matches!(
            self,
            Self::Database(rusqlite::Error::SqliteFailure(error, _))
                if matches!(
                    error.code,
                    rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
                )
        )
    }
}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(message) => write!(formatter, "configuration error: {message}"),
            Self::Database(error) => write!(formatter, "database error: {error}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Serialization(error) => write!(formatter, "serialization error: {error}"),
        }
    }
}

impl std::error::Error for ApplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::Configuration(_) => None,
        }
    }
}

impl From<rusqlite::Error> for ApplicationError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<std::io::Error> for ApplicationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ApplicationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

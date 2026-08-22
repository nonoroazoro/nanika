use std::fmt::{Display, Formatter};

/// Error at the application extension boundary.
#[derive(Debug)]
pub enum ApplicationError {
    Configuration(String),
    Database(rusqlite::Error),
    Io(std::io::Error),
    Serialization(serde_json::Error),
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

impl std::error::Error for ApplicationError {}

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

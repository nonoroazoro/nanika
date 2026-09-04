use std::io;

/// Configuration boundary errors.
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Parse(String),
    Serialize(serde_json::Error),
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "configuration I/O error: {error}"),
            Self::Parse(error) => write!(formatter, "configuration parse error: {error}"),
            Self::Serialize(error) => {
                write!(formatter, "configuration serialization error: {error}")
            }
            Self::Invalid(error) => write!(formatter, "invalid configuration: {error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

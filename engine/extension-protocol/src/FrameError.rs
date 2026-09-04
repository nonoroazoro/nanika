use std::io;

/// Errors raised while framing or decoding protocol messages.
#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    InvalidLength(usize),
    Json(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::InvalidLength(length) => write!(formatter, "invalid frame length: {length}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

use std::io;

use nanika_protocol::FrameError;

/// Failures raised by the extension process supervisor.
#[derive(Debug)]
pub enum SupervisorError {
    Io(io::Error),
    Protocol(FrameError),
    ChannelClosed,
    Cancelled(&'static str),
    UnexpectedMessage(String),
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Protocol(error) => write!(formatter, "protocol error: {error}"),
            Self::ChannelClosed => write!(formatter, "extension protocol channel closed"),
            Self::Cancelled(operation) => {
                write!(formatter, "extension cancelled during {operation}")
            }
            Self::UnexpectedMessage(message) => write!(formatter, "unexpected message: {message}"),
        }
    }
}

impl std::error::Error for SupervisorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::ChannelClosed | Self::Cancelled(_) | Self::UnexpectedMessage(_) => None,
        }
    }
}

impl From<io::Error> for SupervisorError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FrameError> for SupervisorError {
    fn from(error: FrameError) -> Self {
        Self::Protocol(error)
    }
}

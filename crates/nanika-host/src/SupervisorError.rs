use std::io;

use nanika_protocol::FrameError;

/// Failures raised by the extension process supervisor.
#[derive(Debug)]
pub enum SupervisorError {
    Io(io::Error),
    Protocol(FrameError),
    Timeout(&'static str),
    ChannelClosed,
    Cancelled(&'static str),
    QueueFull,
    UnexpectedMessage(String),
    RestartLimit,
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Protocol(error) => write!(formatter, "protocol error: {error}"),
            Self::Timeout(operation) => write!(formatter, "extension timed out during {operation}"),
            Self::ChannelClosed => write!(formatter, "extension protocol channel closed"),
            Self::Cancelled(operation) => {
                write!(formatter, "extension cancelled during {operation}")
            }
            Self::QueueFull => write!(formatter, "extension action queue is full"),
            Self::UnexpectedMessage(message) => write!(formatter, "unexpected message: {message}"),
            Self::RestartLimit => write!(formatter, "extension restart limit reached"),
        }
    }
}

impl std::error::Error for SupervisorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::Timeout(_)
            | Self::ChannelClosed
            | Self::Cancelled(_)
            | Self::QueueFull
            | Self::UnexpectedMessage(_)
            | Self::RestartLimit => None,
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

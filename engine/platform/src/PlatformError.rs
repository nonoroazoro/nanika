/// Platform-independent adapter error.
#[derive(Debug)]
pub enum PlatformError {
    Unsupported(&'static str),
    Io(std::io::Error),
    OsCode { operation: &'static str, code: u32 },
    Timeout(&'static str),
    ActivationChannelClosed,
    EventChannelClosed(&'static str),
    QueueFull(&'static str),
    Message(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(operation) => {
                write!(formatter, "unsupported platform operation: {operation}")
            }
            Self::Io(error) => write!(formatter, "platform I/O error: {error}"),
            Self::OsCode { operation, code } => {
                write!(formatter, "{operation} failed with OS error {code}")
            }
            Self::Timeout(operation) => write!(formatter, "platform timed out during {operation}"),
            Self::ActivationChannelClosed => write!(formatter, "activation channel closed"),
            Self::EventChannelClosed(owner) => write!(formatter, "{owner} event channel closed"),
            Self::QueueFull(owner) => write!(formatter, "{owner} queue is full"),
            Self::Message(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PlatformError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Unsupported(_)
            | Self::OsCode { .. }
            | Self::Timeout(_)
            | Self::ActivationChannelClosed
            | Self::EventChannelClosed(_)
            | Self::QueueFull(_)
            | Self::Message(_) => None,
        }
    }
}

impl From<std::io::Error> for PlatformError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

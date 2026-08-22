/// Platform-independent adapter error.
#[derive(Debug)]
pub enum PlatformError {
    Unsupported(&'static str),
    Io(std::io::Error),
    OsCode { operation: &'static str, code: u32 },
    Timeout(&'static str),
    Hotkey(global_hotkey::Error),
    ActivationChannelClosed,
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
            Self::Hotkey(error) => write!(formatter, "hotkey error: {error}"),
            Self::ActivationChannelClosed => write!(formatter, "activation channel closed"),
        }
    }
}

impl std::error::Error for PlatformError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Hotkey(error) => Some(error),
            Self::Unsupported(_)
            | Self::OsCode { .. }
            | Self::Timeout(_)
            | Self::ActivationChannelClosed => None,
        }
    }
}

impl From<std::io::Error> for PlatformError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

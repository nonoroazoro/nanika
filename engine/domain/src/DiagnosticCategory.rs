/// Stable category for one host-owned diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticCategory {
    Configuration,
    Extension,
    Internal,
    Launch,
    Permission,
    Platform,
    Storage,
}

impl DiagnosticCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Extension => "extension",
            Self::Internal => "internal",
            Self::Launch => "launch",
            Self::Permission => "permission",
            Self::Platform => "platform",
            Self::Storage => "storage",
        }
    }
}

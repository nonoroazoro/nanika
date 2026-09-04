use crate::DiagnosticCategory;

/// Stable code for one host-owned diagnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticCode {
    ConfigurationUnavailable,
    DiagnosticsUnavailable,
    ExtensionUnavailable,
    InternalFailure,
    LaunchFailed,
    PermissionDenied,
    PlatformUnavailable,
    StorageUnavailable,
}

impl DiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfigurationUnavailable => "host.configuration.unavailable",
            Self::DiagnosticsUnavailable => "host.diagnostics.unavailable",
            Self::ExtensionUnavailable => "host.extension.unavailable",
            Self::InternalFailure => "host.internal.failure",
            Self::LaunchFailed => "host.launch.failed",
            Self::PermissionDenied => "host.permission.denied",
            Self::PlatformUnavailable => "host.platform.unavailable",
            Self::StorageUnavailable => "host.storage.unavailable",
        }
    }

    pub const fn category(self) -> DiagnosticCategory {
        match self {
            Self::ConfigurationUnavailable => DiagnosticCategory::Configuration,
            Self::DiagnosticsUnavailable | Self::InternalFailure => DiagnosticCategory::Internal,
            Self::ExtensionUnavailable => DiagnosticCategory::Extension,
            Self::LaunchFailed => DiagnosticCategory::Launch,
            Self::PermissionDenied => DiagnosticCategory::Permission,
            Self::PlatformUnavailable => DiagnosticCategory::Platform,
            Self::StorageUnavailable => DiagnosticCategory::Storage,
        }
    }
}

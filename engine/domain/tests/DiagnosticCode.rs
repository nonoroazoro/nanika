use std::collections::HashSet;

use nanika_core::{DiagnosticCategory, DiagnosticCode};

#[test]
fn diagnostic_codes_are_unique_and_stably_categorized() {
    let codes = [
        (
            DiagnosticCode::ConfigurationUnavailable,
            DiagnosticCategory::Configuration,
        ),
        (
            DiagnosticCode::DiagnosticsUnavailable,
            DiagnosticCategory::Internal,
        ),
        (
            DiagnosticCode::ExtensionUnavailable,
            DiagnosticCategory::Extension,
        ),
        (
            DiagnosticCode::InternalFailure,
            DiagnosticCategory::Internal,
        ),
        (DiagnosticCode::LaunchFailed, DiagnosticCategory::Launch),
        (
            DiagnosticCode::PermissionDenied,
            DiagnosticCategory::Permission,
        ),
        (
            DiagnosticCode::PlatformUnavailable,
            DiagnosticCategory::Platform,
        ),
        (
            DiagnosticCode::StorageUnavailable,
            DiagnosticCategory::Storage,
        ),
    ];
    let unique = codes
        .into_iter()
        .map(|(code, _)| code.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(unique.len(), codes.len());
    for (code, category) in codes {
        assert_eq!(code.category(), category);
    }
}

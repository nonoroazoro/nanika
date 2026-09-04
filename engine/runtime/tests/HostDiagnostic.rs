use std::error::Error;

use crate::host_diagnostic::should_record;
use crate::{DiagnosticCategory, DiagnosticCode, HostDiagnostic};

#[test]
fn diagnostic_display_and_debug_redact_the_technical_source() {
    let secret = "query=private clipboard payload";
    let diagnostic = HostDiagnostic::from_message(
        DiagnosticCode::ExtensionUnavailable,
        "initialize extension",
        "An extension could not start. Open diagnostics for details.",
        secret,
    );

    assert_eq!(
        diagnostic.to_string(),
        "An extension could not start. Open diagnostics for details."
    );
    let debug = format!("{diagnostic:?}");
    assert!(!debug.contains(secret));
    assert!(!debug.contains("An extension could not start"));
    assert_eq!(
        diagnostic.source().map(ToString::to_string).as_deref(),
        Some(secret)
    );
    assert_eq!(
        diagnostic
            .clone()
            .source()
            .map(ToString::to_string)
            .as_deref(),
        Some(secret)
    );
}

#[test]
fn diagnostic_codes_have_stable_categories() {
    assert_eq!(
        DiagnosticCode::StorageUnavailable.category(),
        DiagnosticCategory::Storage
    );
    assert_eq!(
        DiagnosticCode::StorageUnavailable.as_str(),
        "host.storage.unavailable"
    );
}

#[test]
fn repeated_events_are_suppressed_inside_the_record_interval() {
    let diagnostic = HostDiagnostic::new(
        DiagnosticCode::InternalFailure,
        "test repeated diagnostic",
        "A test operation failed.",
    )
    .with_safe_context("test-context");

    assert!(should_record(&diagnostic, false));
    assert!(!should_record(&diagnostic, false));
    assert!(should_record(&diagnostic, true));
}

#[test]
fn distinct_safe_contexts_are_recorded_independently() {
    let application = HostDiagnostic::new(
        DiagnosticCode::ExtensionUnavailable,
        "test extension contexts",
        "Some features are unavailable.",
    )
    .with_safe_context("com.nanika.application");
    let calculator = HostDiagnostic::new(
        DiagnosticCode::ExtensionUnavailable,
        "test extension contexts",
        "Some features are unavailable.",
    )
    .with_safe_context("com.nanika.calculator");

    assert!(should_record(&application, false));
    assert!(should_record(&calculator, false));
}

use std::error::Error;

use crate::{DiagnosticCode, HostDiagnostic};

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

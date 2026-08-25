use crate::host_app::{
    extension_startup_user_message, maximum_visible_result_index, should_render_runtime_error,
    truncate_chars,
};
use crate::{DiagnosticCode, ExtensionStartupError, HostDiagnostic};

#[test]
fn selection_stays_within_rendered_results() {
    assert_eq!(maximum_visible_result_index(0), 0);
    assert_eq!(maximum_visible_result_index(3), 2);
    assert_eq!(maximum_visible_result_index(100), 7);
}

#[test]
fn query_truncation_preserves_utf8() {
    let mut query = "一二三four".to_owned();
    truncate_chars(&mut query, 4);
    assert_eq!(query, "一二三f");
}

#[test]
fn repeated_runtime_messages_render_once() {
    let errors = vec![
        HostDiagnostic::new(
            DiagnosticCode::ExtensionUnavailable,
            "start extension",
            "Some Nanika features are unavailable.",
        ),
        HostDiagnostic::new(
            DiagnosticCode::ExtensionUnavailable,
            "start extension",
            "Some Nanika features are unavailable.",
        ),
        HostDiagnostic::new(
            DiagnosticCode::StorageUnavailable,
            "start storage",
            "History is unavailable.",
        ),
    ];

    assert!(should_render_runtime_error(&errors, 0));
    assert!(!should_render_runtime_error(&errors, 1));
    assert!(should_render_runtime_error(&errors, 2));
    assert!(!should_render_runtime_error(&errors, 3));
}

#[test]
fn extension_startup_message_names_user_features() {
    let errors = vec![
        ExtensionStartupError::new("com.nanika.application", "missing binary"),
        ExtensionStartupError::new("com.nanika.calculator", "missing binary"),
    ];

    assert_eq!(
        extension_startup_user_message(&errors),
        "App search and calculator are unavailable. Restart Nanika. If the problem continues, reinstall Nanika or the affected add-on."
    );
}

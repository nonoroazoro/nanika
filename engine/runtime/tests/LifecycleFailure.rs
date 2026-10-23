use crate::{LifecycleFailure, MAX_LIFECYCLE_DIAGNOSTIC_BYTES, OperationErrorCode};

#[test]
fn process_diagnostics_are_bounded_at_utf8_boundaries_and_redacted_in_debug() {
    let failure = LifecycleFailure::new(OperationErrorCode::ExtensionExited).with_diagnostic(Some(
        format!("{}tail-secret", "界".repeat(MAX_LIFECYCLE_DIAGNOSTIC_BYTES))
    ));
    let diagnostic = failure.diagnostic.as_ref().unwrap();
    assert!(diagnostic.starts_with("[earlier diagnostic omitted]\n"));
    assert!(diagnostic.ends_with("tail-secret"));
    assert!(diagnostic.len() <= MAX_LIFECYCLE_DIAGNOSTIC_BYTES);
    assert!(!format!("{failure:?}").contains("tail-secret"));
    assert!(
        serde_json::to_value(&failure).unwrap()["diagnostic"]
            .as_str()
            .unwrap()
            .ends_with("tail-secret")
    );
}

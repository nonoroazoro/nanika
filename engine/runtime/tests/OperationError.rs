use crate::{OperationError, OperationErrorCode, SupervisorError};

#[test]
fn classification_uses_the_failure_variant_and_preserves_original_detail() {
    for (failure, code) in [
        (
            SupervisorError::ChannelClosed,
            OperationErrorCode::ExtensionUnavailable
        ),
        (
            SupervisorError::Cancelled("text read"),
            OperationErrorCode::Cancelled
        ),
        (
            SupervisorError::Terminated("text read"),
            OperationErrorCode::ExtensionTerminated
        ),
        (
            SupervisorError::Io(std::io::Error::other("cancelled")),
            OperationErrorCode::ExtensionIo
        ),
        (
            SupervisorError::UnexpectedMessage("cancelled".into()),
            OperationErrorCode::ExtensionProtocol
        )
    ] {
        let expected = failure.to_string();
        let converted = OperationError::from(failure);
        assert_eq!(converted, OperationError::with_detail(code, expected));
    }
}

#[test]
fn retained_presentation_clones_without_logging_original_user_text() {
    let prepared = nanika_localization::PreparedError::new(
        nanika_protocol::ProtocolError::new("external", "private text")
            .with_detail("private detail"),
        None,
        "macos"
    )
    .unwrap();
    let original = SupervisorError::Reported(Box::new(prepared));
    let retained = OperationError::from(&original);
    assert!(!format!("{retained:?}").contains("private"));
    let encoded = serde_json::to_value(retained.clone()).unwrap();
    assert_eq!(encoded["error"]["messages"]["en"], "private text");
    assert_eq!(encoded["error"]["error"]["detail"], "private detail");
    assert!(matches!(retained, OperationError::Extension { .. }));
    assert!(matches!(original, SupervisorError::Reported(_)));
}

#[test]
fn cleanup_failure_preserves_both_diagnostics_without_logging_them() {
    let error = OperationError::Cleanup {
        primary: Box::new(OperationError::with_detail(
            OperationErrorCode::NavigationFailed,
            "private primary"
        )),
        cleanup: Box::new(OperationError::Extension {
            error: Box::new(
                nanika_localization::PreparedError::new(
                    nanika_protocol::ProtocolError::new("close_failed", "Cannot close view")
                        .with_detail("private cleanup"),
                    None,
                    "macos"
                )
                .unwrap()
            )
        })
    };
    assert!(!format!("{error:?}").contains("private"));
    let value = serde_json::to_value(error).unwrap();
    assert_eq!(value["kind"], "cleanup");
    assert_eq!(value["primary"]["detail"], "private primary");
    assert_eq!(
        value["cleanup"]["error"]["error"]["detail"],
        "private cleanup"
    );
    assert_eq!(
        value["cleanup"]["error"]["messages"]["en"],
        "Cannot close view"
    );
}

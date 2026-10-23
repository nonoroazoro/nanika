use nanika_protocol::{Message, MessageArgument, ProtocolError, message};
use serde_json::json;

#[test]
fn failures_preserve_machine_codes_references_arguments_and_original_details() {
    let error = ProtocolError::new(
        "copy.failed",
        message(
            "copy-failed",
            [("name".into(), MessageArgument::Text("notes.txt".into()))]
        )
    )
    .with_detail("OS error 5: access denied");
    error.validate().unwrap();
    let frame = Message::Error {
        request_id: Some("action-7".into()),
        error
    };
    let encoded = serde_json::to_value(&frame).unwrap();
    assert_eq!(
        encoded,
        json!({
            "type": "error", "request_id": "action-7",
            "error": { "code": "copy.failed", "message": { "message": "copy-failed", "args": { "name": "notes.txt" } }, "detail": "OS error 5: access denied" }
        })
    );
    let decoded: Message = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    assert!(
        serde_json::from_value::<Message>(
            json!({"type":"error", "request_id": null, "code":"old", "message":"old wire shape"})
        )
        .is_err()
    );
    let mut foreign = encoded;
    foreign["error"]["message"]["scope"] = json!("host");
    assert!(serde_json::from_value::<Message>(foreign).is_err());
}

#[test]
fn diagnostic_formatting_does_not_log_reference_argument_values_by_default() {
    let error = ProtocolError::new(
        "copy_failed",
        message(
            "copy-failed",
            [("path".into(), MessageArgument::Text("private-path".into()))]
        )
    )
    .with_detail("original diagnostic");
    let debug = format!("{error:?}");
    assert!(debug.contains("copy-failed"));
    assert!(!debug.contains("private-path"));
    assert!(!debug.contains("original diagnostic"));
    assert!(!error.to_string().contains("private-path"));
    assert!(error.to_string().contains("original diagnostic"));
}

#[test]
fn message_and_diagnostic_bounds_reject_without_truncating() {
    for code in [
        "".to_owned(),
        "bad code".into(),
        "错误".into(),
        "a".repeat(129)
    ] {
        assert!(ProtocolError::new(code, "Failure").validate().is_err());
    }
    ProtocolError::new(
        "A.b-c_0",
        "界".repeat(nanika_protocol::MAX_ERROR_MESSAGE_CHARS)
    )
    .validate()
    .unwrap();
    for value in [
        "".to_owned(),
        " \n".into(),
        "bad\0text".into(),
        "bad\u{1f}text".into(),
        "bad\u{7f}text".into(),
        "bad\u{85}text".into(),
        "界".repeat(nanika_protocol::MAX_ERROR_MESSAGE_CHARS + 1)
    ] {
        assert!(ProtocolError::new("failure", value).validate().is_err());
    }
    let exact = ProtocolError::new("failure", "Failed")
        .with_detail("x".repeat(nanika_protocol::MAX_ERROR_DETAIL_BYTES));
    exact.validate().unwrap();
    for value in [
        "bad\0detail".to_owned(),
        "x".repeat(nanika_protocol::MAX_ERROR_DETAIL_BYTES + 1)
    ] {
        let error = ProtocolError::new("failure", "Failed").with_detail(value.clone());
        assert!(error.validate().is_err());
        assert_eq!(error.detail.as_deref(), Some(value.as_str()));
    }
    assert!(
        ProtocolError::new(
            "failure",
            message(
                "failed",
                [("platform".into(), MessageArgument::Text("windows".into()))]
            )
        )
        .validate()
        .is_err()
    );
}

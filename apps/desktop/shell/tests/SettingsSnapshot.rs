use crate::{SaveSettingsRequest, validate_settings_request};

#[test]
fn settings_request_has_no_fixed_value_byte_quota() {
    let request = SaveSettingsRequest {
        extension_id: "test.extension".into(),
        key: "entries".into(),
        value: serde_json::json!(vec!["x".repeat(4000); 2400])
    };
    assert!(serde_json::to_vec(&request.value).unwrap().len() > 8 * 1024 * 1024);
    validate_settings_request(&request).unwrap();
    let invalid = SaveSettingsRequest {
        key: String::new(),
        ..request
    };
    assert_eq!(
        validate_settings_request(&invalid).unwrap_err(),
        nanika_host::OperationError::new(nanika_host::OperationErrorCode::InvalidConfiguration)
    );
}

#[test]
fn lost_configuration_receipt_preserves_the_cause_without_inventing_transaction_facts() {
    let result: Result<nanika_host::ConfigurationSaveOutcome, nanika_host::OperationError> =
        Err(nanika_host::OperationError::with_detail(
            nanika_host::OperationErrorCode::ConfigurationUnconfirmed,
            "EPIPE: original detail"
        ));
    let event = serde_json::to_value(crate::SettingsSaveResult::from(result)).unwrap();
    assert_eq!(
        event,
        serde_json::json!({
            "status": "failed",
            "error": { "code": "unconfirmed", "cause": {
                "kind": "host", "code": "configurationUnconfirmed", "detail": "EPIPE: original detail"
            }}
        })
    );
}

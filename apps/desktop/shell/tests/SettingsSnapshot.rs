use crate::{SaveSettingsRequest, validate_settings_request};

#[test]
fn settings_request_has_no_fixed_value_byte_quota() {
    let request = SaveSettingsRequest {
        extension_id: "test.extension".into(),
        key: "entries".into(),
        value: serde_json::json!(vec!["x".repeat(4000); 2400]),
    };
    assert!(serde_json::to_vec(&request.value).unwrap().len() > 8 * 1024 * 1024);
    validate_settings_request(&request).unwrap();
    let invalid = SaveSettingsRequest {
        key: String::new(),
        ..request
    };
    assert!(validate_settings_request(&invalid).is_err());
}

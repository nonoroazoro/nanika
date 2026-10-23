use nanika_protocol::{LocalizedText, MessageReference, message};
use serde_json::json;

#[test]
fn reference_constructor_has_no_locale_state_and_round_trips_plain_json() {
    let text = message(
        "files-more",
        [
            ("name".into(), "report.pdf".into()),
            ("count".into(), 2i64.into())
        ]
    );
    let wire = serde_json::to_value(&text).unwrap();
    assert_eq!(
        wire,
        json!({"message":"files-more","args":{"name":"report.pdf","count":2}})
    );
    assert_eq!(serde_json::from_value::<LocalizedText>(wire).unwrap(), text);
    assert_eq!(
        serde_json::to_value(message("extension-name", [])).unwrap(),
        json!({"message":"extension-name"})
    );
    assert_eq!(
        serde_json::to_value(LocalizedText::from("report.pdf")).unwrap(),
        json!("report.pdf")
    );
}

#[test]
fn objects_cannot_carry_namespace_overrides_or_nested_arguments() {
    for value in [
        json!({"message":"hello","namespace":"host"}),
        json!({"message":"hello","args":{"count":1.5}}),
        json!({"message":"hello","args":{"name":{"message":"other"}}}),
        json!({"message":"hello","args":{"name":true}}),
        json!({"message":"hello","args":{"name":null}})
    ] {
        assert!(serde_json::from_value::<LocalizedText>(value).is_err());
    }
    for value in [
        json!({"message":"hello.title"}),
        json!({"message":"host/hello"}),
        json!({"message":"hello","args":{"platform":"macos"}}),
        json!({"message":"hello","args":{"count":9007199254740992i64}})
    ] {
        assert!(
            serde_json::from_value::<MessageReference>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}

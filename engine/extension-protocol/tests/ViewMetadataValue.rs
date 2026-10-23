use nanika_protocol::{View, ViewMetadataValue};
use serde_json::json;

#[test]
fn metadata_bytes_round_trip_without_becoming_a_message_or_string() {
    for bytes in [
        0,
        1023,
        1024,
        1536,
        16_777_216,
        nanika_protocol::MAX_MESSAGE_INTEGER as u64
    ] {
        let source = json!({"bytes": bytes});
        let value: ViewMetadataValue = serde_json::from_value(source.clone()).unwrap();
        assert_eq!(value, ViewMetadataValue::Bytes { bytes });
        assert!(serde_json::from_value::<nanika_protocol::LocalizedText>(source.clone()).is_err());
        assert_eq!(serde_json::to_value(&value).unwrap(), source);
        let view = _view(source);
        view.validate().unwrap();
        let mut texts = Vec::new();
        view.visit_texts(&mut |field, text, _, _| {
            texts.push((field.to_owned(), text.clone()));
            Ok(())
        })
        .unwrap();
        assert_eq!(texts, [("detail metadata title".into(), "Size".into())]);
    }
}

#[test]
fn unsafe_or_malformed_metadata_is_rejected_before_publication() {
    for bytes in [nanika_protocol::MAX_MESSAGE_INTEGER as u64 + 1, u64::MAX] {
        assert!(_view(json!({"bytes": bytes})).validate().is_err());
    }
    for value in [
        json!({"bytes": -1}),
        json!({"bytes": 0.5}),
        json!({"bytes": "1024"}),
        json!({"bytes": null}),
        json!({"bytes": 10, "message": "size"}),
        json!({"bytes": 10, "unit": "KB"}),
        json!({"kilobytes": 1}),
        json!(10)
    ] {
        assert!(
            serde_json::from_value::<ViewMetadataValue>(value.clone()).is_err(),
            "{value}"
        );
    }
    for value in [
        json!("x".repeat(2049)),
        json!("bad\u{0}"),
        json!({"message": "invalid key"})
    ] {
        assert!(_view(value).validate().is_err());
    }
}

#[test]
fn text_metadata_retains_the_existing_resource_scope_and_wire_shape() {
    for source in [
        json!("user content"),
        json!({"message": "size", "args": {"count": 2}})
    ] {
        let view = _view(source.clone());
        view.validate().unwrap();
        let mut values = Vec::new();
        view.visit_texts(&mut |field, text, _, _| {
            if field == "detail metadata value" {
                values.push(serde_json::to_value(text).unwrap());
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(values, [source]);
    }
}

fn _view(value: serde_json::Value) -> View {
    serde_json::from_value(json!({"kind": "detail", "detail": {
        "title": null,
        "content": {"kind": "files", "files": [{"path": "/file.txt", "name": "file.txt", "icon": null}]},
        "metadata": [{"title": "Size", "value": value}], "actions": []
    }})).unwrap()
}

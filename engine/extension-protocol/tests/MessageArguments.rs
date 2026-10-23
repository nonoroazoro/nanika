use nanika_protocol::{MAX_MESSAGE_ARGUMENTS, MessageArguments, MessageReference, message};

#[test]
fn arguments_retain_ordering_duplicate_key_policy_and_object_wire_format() {
    let arguments: MessageArguments = [
        ("name".into(), "first".into()),
        ("count".into(), 2i64.into()),
        ("name".into(), "last".into())
    ]
    .into_iter()
    .collect();
    assert_eq!(arguments["name"], "last".into());
    assert_eq!(
        arguments.keys().map(String::as_str).collect::<Vec<_>>(),
        ["count", "name"]
    );
    assert_eq!(
        serde_json::to_string(&arguments).unwrap(),
        r#"{"count":2,"name":"last"}"#
    );
    let decoded: MessageArguments =
        serde_json::from_str(r#"{"name":"first","count":2,"name":"last"}"#).unwrap();
    assert_eq!(decoded, arguments);
    assert_eq!(arguments.clone(), arguments);
    assert_eq!(arguments.get("missing"), None);
    assert!(MessageArguments::default().is_empty());
}

#[test]
fn both_wire_admission_and_constructor_validation_enforce_argument_count() {
    for count in [MAX_MESSAGE_ARGUMENTS, MAX_MESSAGE_ARGUMENTS + 1] {
        let text = message(
            "many",
            (0..count).map(|index| (format!("arg-{index}"), 1i64.into()))
        );
        let wire = serde_json::to_value(&text).unwrap();
        let decoded = serde_json::from_value::<MessageReference>(wire);
        if count == MAX_MESSAGE_ARGUMENTS {
            decoded.unwrap().validate().unwrap();
        } else {
            assert!(decoded.is_err());
            let nanika_protocol::LocalizedText::Message(reference) = text else {
                panic!("reference")
            };
            assert!(reference.validate().is_err());
        }
    }
}

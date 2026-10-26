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

#[test]
fn supplied_translations_round_trip_and_validate_every_locale() {
    let wire = json!({"fallback":"NeteaseMusic","translations":{"en":"NetEaseMusic","zh-Hans":"网易云音乐"}});
    let text: LocalizedText = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&text).unwrap(), wire);
    text.validate_literals(|value| {
        assert!(!value.is_empty());
        Ok(())
    })
    .unwrap();
    let LocalizedText::Translated(mut value) = text else {
        panic!("expected domain translations")
    };
    assert_eq!(value.resolve("zh-Hans"), "网易云音乐");
    assert_eq!(value.resolve("ja"), "NetEaseMusic");
    value.translations.remove("en");
    assert_eq!(value.resolve("ja"), "NeteaseMusic");
    value.translations.insert("ja".into(), "bad\0".into());
    assert!(
        LocalizedText::Translated(value)
            .validate_literals(|value| {
                if value.contains('\0') {
                    Err("invalid control".into())
                } else {
                    Ok(())
                }
            })
            .is_err()
    );
}

#[test]
fn supplied_translations_reject_invalid_shape_tags_and_unbounded_data() {
    for wire in [
        json!({"fallback":"Name","translations":{},"message":"name"}),
        json!({"fallback":"Name","translations":{"en":null}}),
        json!({"fallback":"Name","translations":[]})
    ] {
        assert!(serde_json::from_value::<LocalizedText>(wire).is_err());
    }
    for translations in [
        json!({}),
        json!({"zh_hans":"Name"}),
        json!({"EN":"Name"}),
        json!({"eng":"Name"}),
        json!({"iw":"Name"}),
        json!({"en-UK":"Name"}),
        json!({"en":"x".repeat(65536)})
    ] {
        let text: LocalizedText =
            serde_json::from_value(json!({"fallback":"Name","translations":translations})).unwrap();
        assert!(text.validate_literals(|_| Ok(())).is_err());
    }
}

#[test]
fn language_matching_preserves_scripts_regions_and_foreign_content() {
    let mut text = nanika_protocol::TranslatedText {
        fallback: "音楽箱".into(),
        translations: [
            ("en-GB".into(), "Music".into()),
            ("zh-TW".into(), "網易雲音樂".into()),
            ("ja".into(), "音楽箱".into())
        ]
        .into()
    };
    assert_eq!(text.resolve("zh-Hans"), "網易雲音樂");
    assert_eq!(text.resolve("en"), "Music");
    assert_eq!(text.resolve("ja"), "音楽箱");
    assert_eq!(text.resolve("fr"), "Music");
    text.translations
        .insert("zh-CN".into(), "网易云音乐".into());
    assert_eq!(text.resolve("zh-Hans"), "网易云音乐");
    assert_eq!(text.resolve("zh-Hant"), "網易雲音樂");
    text.translations.retain(|locale, _| locale == "ja");
    assert_eq!(text.resolve("en"), "音楽箱");
    assert_eq!(text.resolve("zh-Hans"), "音楽箱");
    text.translations = (1..=64)
        .map(|index| (format!("en-var{index:04}"), format!("Name {index}")))
        .collect();
    text.validate().unwrap();
}

#[test]
fn source_aliases_are_normalized_before_protocol_admission() {
    for (source, expected) in [
        ("eng", "en"),
        ("jpn", "ja"),
        ("iw", "he"),
        ("en-UK", "en-GB")
    ] {
        assert_eq!(
            nanika_protocol::canonical_language_tag(source).as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn translated_text_matches_shared_frontend_conformance_cases() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/translated-text.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let text: nanika_protocol::TranslatedText =
            serde_json::from_value(case["text"].clone()).unwrap();
        if let Some(expected) = case["expected"].as_str() {
            text.validate().unwrap();
            assert_eq!(
                text.resolve(case["locale"].as_str().unwrap()),
                expected,
                "{}",
                case["name"]
            );
        } else {
            assert!(text.validate().is_err(), "{}", case["name"]);
        }
    }
}

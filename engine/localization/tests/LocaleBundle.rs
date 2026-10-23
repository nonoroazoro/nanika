use nanika_localization::{LocaleBundle, SupportedLocale};
use nanika_protocol::{LocalizedText, message};

#[test]
fn shared_runtime_conformance() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/conformance.json")).unwrap();
    for (locale, source) in [
        (SupportedLocale::English, include_str!("fixtures/en.ftl")),
        (
            SupportedLocale::SimplifiedChinese,
            include_str!("fixtures/zh-Hans.ftl")
        )
    ] {
        let bundle = LocaleBundle::new(locale, [("fixture.ftl".into(), source.into())]).unwrap();
        assert_eq!(
            serde_json::to_value(bundle.contracts()).unwrap(),
            fixtures["contracts"]
        );
        for case in fixtures["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["locale"] == locale.as_str())
        {
            let text: LocalizedText = serde_json::from_value(case["text"].clone()).unwrap();
            let actual = bundle
                .format(&text, case["platform"].as_str().unwrap(), 2048, false)
                .unwrap();
            assert_eq!(
                actual,
                case.get("rustExpected")
                    .unwrap_or(&case["expected"])
                    .as_str()
                    .unwrap(),
                "{case}"
            );
        }
    }
}

#[test]
fn rejects_bad_references_arguments_and_destination_overflow_without_leaking_values() {
    let bundle = LocaleBundle::new(
        SupportedLocale::English,
        [("fixture.ftl".into(), include_str!("fixtures/en.ftl").into())]
    )
    .unwrap();
    for text in [
        message("unknown", []),
        message("integer", []),
        message("brand", [("unexpected".into(), "private content".into())]),
        message("integer", [("count".into(), i64::MAX.into())]),
        message("platform", [("platform".into(), "windows".into())]),
        message("content", [("value".into(), "x".repeat(2049).into())])
    ] {
        let error = bundle.format(&text, "macos", 2048, false).unwrap_err();
        assert!(!error.contains("private content"));
    }
    assert!(
        bundle
            .format(&message("brand", []), "macos", 2, false)
            .is_err()
    );
    assert!(bundle.format(&"".into(), "macos", 20, false).is_err());
    assert!(bundle.format(&"".into(), "macos", 20, true).is_ok());
    assert_eq!(
        bundle.format(&"😀".into(), "macos", 1, false).unwrap(),
        "😀"
    );
    assert!(bundle.format(&"😀😀".into(), "macos", 1, false).is_err());
    assert!(bundle.format(&"\u{85}".into(), "macos", 10, false).is_err());
}

#[test]
fn immutable_bundle_can_be_shared_across_threads() {
    let bundle = std::sync::Arc::new(
        LocaleBundle::new(
            SupportedLocale::English,
            [("fixture.ftl".into(), "hello = Hello".into())]
        )
        .unwrap()
    );
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let bundle = bundle.clone();
            std::thread::spawn(move || {
                for _ in 0..100 {
                    assert_eq!(
                        bundle
                            .format(&message("hello", []), "windows", 128, false)
                            .unwrap(),
                        "Hello"
                    );
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

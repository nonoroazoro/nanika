use nanika_localization::{ResourceSet, SupportedLocale, typescript_message_type};

#[test]
fn type_generation_uses_transitive_contracts_and_excludes_terms_and_platform() {
    let resources = ResourceSet::new(
        SupportedLocale::English,
        [(
            SupportedLocale::English,
            vec![(
                "en.ftl".into(),
                "-brand = Nanika\nplain = { -brand }\ncount = { $count }\nindirect = { count }\nplatform = { $platform ->\n [macos] { $name }\n *[windows] { $count }\n}\n".into()
            )]
        )]
        .into(),
        false
    )
    .unwrap();
    let generated = typescript_message_type(&resources);
    assert!(generated.contains("message: \"plain\"; args?: Record<string, never>"));
    assert!(generated.contains("message: \"indirect\"; args: { \"count\": string | number;"));
    assert!(generated.contains(
        "message: \"platform\"; args: { \"count\": string | number; \"name\": string | number;"
    ));
    assert!(!generated.contains("message: \"-brand\""));
    assert!(!generated.contains("\"platform\": string"));
}

#[test]
fn type_generation_uses_the_declared_complete_locale_and_has_deterministic_order() {
    let source = "z-last = { $value }\na-first = First\n";
    let resources = ResourceSet::new(
        SupportedLocale::SimplifiedChinese,
        [
            (
                SupportedLocale::English,
                vec![("en.ftl".into(), "a-first = First\n".into())]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![("zh.ftl".into(), source.into())]
            )
        ]
        .into(),
        false
    )
    .unwrap();
    let generated = typescript_message_type(&resources);
    assert!(generated.find("a-first").unwrap() < generated.find("z-last").unwrap());
    assert!(generated.contains("\"value\": string | number"));
    let empty = ResourceSet::new(
        SupportedLocale::English,
        [(
            SupportedLocale::English,
            vec![("en.ftl".into(), "# No messages\n".into())]
        )]
        .into(),
        false
    )
    .unwrap();
    assert!(typescript_message_type(&empty).ends_with("never;\n"));
}

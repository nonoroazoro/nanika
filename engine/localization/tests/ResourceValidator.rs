use nanika_localization::{
    LocaleBundle, MAX_LOCALE_RESOURCES, MAX_RESOURCE_BYTES, SupportedLocale
};

#[test]
fn rejects_invalid_resources_before_publication() {
    for source in [
        "hello = {",
        "hello = { missing }",
        "hello = { -missing }",
        "hello = { next }\nnext = { hello }",
        "hello = Hello\nhello = Again",
        "-brand = First\n-brand = Second",
        "hello = { NUMBER($count) }",
        "hello = { DATETIME($time) }",
        "hello = { CUSTOM() }",
        "hello = Value\n    .title = Attribute",
        "hello = { other.title }\nother = Other",
        "hello = { 1000 }",
        "hello = { -0 }",
        "hello = { 1.5 }",
        "hello = { -term(count: 1) }\n-term = { $count }",
        "hello = { $count ->\n [1.5] Invalid\n *[other] Other\n }",
        "hello = { -term }\n-term = { $name }",
        "hello = { -term(name: \"Ada\", extra: \"x\") }\n-term = { $name }"
    ] {
        assert!(
            LocaleBundle::new(
                SupportedLocale::English,
                [("invalid.ftl".into(), source.into())]
            )
            .is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn rejects_duplicate_ids_across_files_and_bounds_resource_work() {
    assert!(
        LocaleBundle::new(
            SupportedLocale::English,
            [
                ("a.ftl".into(), "hello = First".into()),
                ("b.ftl".into(), "hello = Second".into())
            ]
        )
        .is_err()
    );
    assert!(
        LocaleBundle::new(
            SupportedLocale::English,
            [("big.ftl".into(), "x".repeat(MAX_RESOURCE_BYTES + 1))]
        )
        .is_err()
    );
    assert!(
        LocaleBundle::new(
            SupportedLocale::English,
            (0..=MAX_LOCALE_RESOURCES)
                .map(|index| (format!("{index}.ftl"), format!("message-{index} = Text")))
        )
        .is_err()
    );
    let mut source = "message-0 = Text\n".to_owned();
    for index in 1..40 {
        source.push_str(&format!("message-{index} = {{ message-{} }}\n", index - 1));
    }
    assert!(LocaleBundle::new(SupportedLocale::English, [("deep.ftl".into(), source)]).is_err());
    let mut source = "message-0 = Text\n".to_owned();
    for index in 1..20 {
        source.push_str(&format!(
            "message-{index} = {{ message-{previous} }}{{ message-{previous} }}\n",
            previous = index - 1
        ));
    }
    assert!(
        LocaleBundle::new(SupportedLocale::English, [("expansion.ftl".into(), source)]).is_err()
    );
}

#[test]
fn rejects_deep_parser_input_and_expanded_placeables_before_formatting() {
    let nested = format!("hello = {} x {}", "{ ".repeat(1000), " }".repeat(1000));
    assert!(LocaleBundle::new(SupportedLocale::English, [("nested.ftl".into(), nested)]).is_err());
    let repeated = format!("-term = Text\nhello = {}", "{ -term }".repeat(101));
    assert!(
        LocaleBundle::new(
            SupportedLocale::English,
            [("expansion.ftl".into(), repeated)]
        )
        .is_err()
    );
    let comment = format!("# {}\nhello = Text", "{".repeat(100));
    assert!(LocaleBundle::new(SupportedLocale::English, [("comment.ftl".into(), comment)]).is_ok());
    assert!(
        LocaleBundle::new(
            SupportedLocale::English,
            [(
                "duplicate.ftl".into(),
                "-same = Term\nsame = Message".into()
            )]
        )
        .is_err()
    );
}

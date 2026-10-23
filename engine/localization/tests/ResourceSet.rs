use nanika_localization::{LocalizationResources, ResourceSet, SupportedLocale};
use nanika_protocol::message;
use std::collections::BTreeMap;

#[test]
fn fallback_keeps_references_inside_one_bundle() {
    let resources = ResourceSet::new(
        SupportedLocale::English,
        BTreeMap::from([
            (
                SupportedLocale::English,
                vec![(
                    "en.ftl".into(),
                    "-brand = English brand\nhello = Hello, { -brand }\nonly-default = Default"
                        .into()
                )]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![(
                    "zh.ftl".into(),
                    "-brand = 中文品牌\nhello = 你好，{ -brand }".into()
                )]
            )
        ]),
        false
    )
    .unwrap();
    assert_eq!(
        resources
            .format(
                SupportedLocale::SimplifiedChinese,
                &message("hello", []),
                "macos",
                128,
                false
            )
            .unwrap(),
        "你好，中文品牌"
    );
    assert_eq!(
        resources
            .format(
                SupportedLocale::SimplifiedChinese,
                &message("only-default", []),
                "macos",
                128,
                false
            )
            .unwrap(),
        "Default"
    );
    assert!(
        resources
            .format(
                SupportedLocale::SimplifiedChinese,
                &message("unknown", []),
                "macos",
                128,
                false
            )
            .is_err()
    );
    assert!(
        resources
            .format(
                SupportedLocale::SimplifiedChinese,
                &message("hello", [("unexpected".into(), "secret".into())]),
                "macos",
                128,
                false
            )
            .is_err()
    );
}

#[test]
fn default_locale_must_be_complete_and_translations_must_agree_on_arguments() {
    for translated in ["hello = { $different }", "extra = Unexpected"] {
        assert!(
            ResourceSet::new(
                SupportedLocale::English,
                BTreeMap::from([
                    (
                        SupportedLocale::English,
                        vec![("en.ftl".into(), "hello = { $name }".into())]
                    ),
                    (
                        SupportedLocale::SimplifiedChinese,
                        vec![("zh.ftl".into(), translated.into())]
                    )
                ]),
                false
            )
            .is_err()
        );
    }
    let sources = BTreeMap::from([(
        SupportedLocale::SimplifiedChinese,
        vec![("zh.ftl".into(), "hello = 你好".into())]
    )]);
    assert!(ResourceSet::new(SupportedLocale::English, sources.clone(), false).is_err());
    assert!(ResourceSet::new(SupportedLocale::SimplifiedChinese, sources.clone(), true).is_err());
    let resources = ResourceSet::new(SupportedLocale::SimplifiedChinese, sources, false).unwrap();
    assert_eq!(
        resources
            .format(
                SupportedLocale::English,
                &message("hello", []),
                "windows",
                128,
                false
            )
            .unwrap(),
        "你好"
    );
}

#[test]
fn content_identity_changes_on_resource_changes_and_retained_sets_stay_valid() {
    let prepare = |text: &str| {
        ResourceSet::new(
            SupportedLocale::English,
            BTreeMap::from([(
                SupportedLocale::English,
                vec![("en.ftl".into(), text.into())]
            )]),
            false
        )
        .unwrap()
    };
    let old = prepare("hello = Before");
    let duplicate = prepare("hello = Before");
    let new = prepare("hello = After");
    assert_eq!(old.prepared().identity, duplicate.prepared().identity);
    assert_ne!(old.prepared().identity, new.prepared().identity);
    assert_eq!(
        old.format(
            SupportedLocale::English,
            &message("hello", []),
            "macos",
            128,
            false
        )
        .unwrap(),
        "Before"
    );
}

#[test]
fn unsafe_paths_and_undeclared_locales_are_rejected() {
    for path in [
        "/tmp/en.ftl",
        "../en.ftl",
        "locales/../en.ftl",
        "C:/en.ftl",
        "https://example.org/en.ftl",
        "locales\\en.ftl",
        "locales//en.ftl",
        "./en.ftl",
        "en.json"
    ] {
        let declaration = LocalizationResources {
            default_locale: SupportedLocale::English,
            resources: BTreeMap::from([(SupportedLocale::English, vec![path.into()])])
        };
        assert!(declaration.validate().is_err(), "accepted {path}");
    }
    assert!(
        serde_json::from_str::<LocalizationResources>(
            r#"{"defaultLocale":"en","resources":{"fr":["fr.ftl"]}}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<LocalizationResources>(
            r#"{"defaultLocale":"en","resources":{"en":["en.ftl"]},"supportedLocales":["en"]}"#
        )
        .is_err()
    );
}

#[test]
fn package_loading_bounds_reads_and_rejects_escaping_symlinks() {
    let root = std::env::temp_dir().join(format!(
        "nanika-localization-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let package = root.join("package");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("en.ftl"), "hello = Hello").unwrap();
    let mut declaration = LocalizationResources {
        default_locale: SupportedLocale::English,
        resources: BTreeMap::from([(SupportedLocale::English, vec!["en.ftl".into()])])
    };
    assert!(ResourceSet::load(&package, &declaration, false).is_ok());
    std::fs::write(
        package.join("en.ftl"),
        "x".repeat(nanika_localization::MAX_RESOURCE_BYTES + 1)
    )
    .unwrap();
    assert!(ResourceSet::load(&package, &declaration, false).is_err());
    std::fs::write(root.join("outside.ftl"), "hello = Outside").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("outside.ftl"), package.join("escape.ftl")).unwrap();
        declaration
            .resources
            .insert(SupportedLocale::English, vec!["escape.ftl".into()]);
        assert!(ResourceSet::load(&package, &declaration, false).is_err());
    }
    declaration
        .resources
        .insert(SupportedLocale::English, vec!["missing.ftl".into()]);
    assert!(ResourceSet::load(&package, &declaration, false).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn translations_can_omit_host_platform_context_without_changing_the_author_contract() {
    let resources = ResourceSet::new(
        SupportedLocale::English,
        BTreeMap::from([
            (
                SupportedLocale::English,
                vec![(
                    "en.ftl".into(),
                    "hello = { $platform ->\n [macos] Log Out\n *[windows] Sign out\n }".into()
                )]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![("zh.ftl".into(), "hello = 退出登录".into())]
            )
        ]),
        true
    )
    .unwrap();
    assert_eq!(
        resources
            .format(
                SupportedLocale::SimplifiedChinese,
                &message("hello", []),
                "windows",
                128,
                false
            )
            .unwrap(),
        "退出登录"
    );
    assert!(
        resources
            .format(
                SupportedLocale::English,
                &message("hello", [("platform".into(), "macos".into())]),
                "windows",
                128,
                false
            )
            .is_err()
    );
}

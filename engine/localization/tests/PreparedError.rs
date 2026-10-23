use nanika_localization::{PreparedError, ResourceSet, SupportedLocale};
use nanika_protocol::{MessageArgument, ProtocolError, message};

#[test]
fn retained_error_switches_languages_after_sender_resources_are_dropped() {
    let original = ProtocolError::new(
        "copy_failed",
        message(
            "failed",
            [("name".into(), MessageArgument::Text("private.txt".into()))]
        )
    )
    .with_detail("EACCES: original OS detail");
    let prepared = {
        let resources = _resources(
            "failed = Could not copy { $name }.",
            "failed = 无法复制 { $name }。"
        );
        PreparedError::new(original.clone(), Some(&resources), "macos").unwrap()
    };
    assert_eq!(prepared.error, original);
    assert_eq!(
        prepared.messages[&SupportedLocale::English],
        "Could not copy \u{2068}private.txt\u{2069}."
    );
    assert_eq!(
        prepared.messages[&SupportedLocale::SimplifiedChinese],
        "无法复制 \u{2068}private.txt\u{2069}。"
    );
    assert!(!format!("{prepared:?}").contains("private.txt"));
    assert!(!format!("{prepared:?}").contains("EACCES"));
    let encoded = serde_json::to_value(prepared).unwrap();
    assert_eq!(encoded["error"]["detail"], "EACCES: original OS detail");
    assert_eq!(
        encoded["messages"]["zh-Hans"],
        "无法复制 \u{2068}private.txt\u{2069}。"
    );
}

#[test]
fn message_admission_requires_owner_resources_and_exact_argument_contracts() {
    let error = ProtocolError::new("failed", message("failed", []));
    assert!(PreparedError::new(error.clone(), None, "macos").is_err());
    for (en, zh) in [
        ("other = Other", "other = 其他"),
        ("failed = Failed { $path }", "failed = 失败 { $path }")
    ] {
        let resources = _resources(en, zh);
        assert!(PreparedError::new(error.clone(), Some(&resources), "macos").is_err());
    }
    let resources = _resources("failed = Failed", "failed = 失败");
    let extra = ProtocolError::new(
        "failed",
        message("failed", [("extra".into(), MessageArgument::Integer(1))])
    );
    assert!(PreparedError::new(extra, Some(&resources), "macos").is_err());
}

#[test]
fn inactive_language_and_platform_are_validated_before_admission() {
    for (en, zh, valid) in [
        ("Failed".to_owned(), "失败".to_owned(), true),
        ("x".repeat(2_048), "失败".into(), true),
        ("Failed".into(), "界".repeat(2_049), false),
        (
            "{ $platform ->\n    [macos] Failed\n   *[windows] { \"\" }\n    }".into(),
            "失败".into(),
            false
        )
    ] {
        let resources = _resources(&format!("failed = {en}"), &format!("failed = {zh}"));
        assert_eq!(
            PreparedError::new(
                ProtocolError::new("failed", message("failed", [])),
                Some(&resources),
                "macos"
            )
            .is_ok(),
            valid
        );
    }
}

#[test]
fn prepared_messages_select_the_receiving_platform() {
    let resources = _resources(
        "failed = { $platform ->\n    [macos] Open System Settings\n   *[windows] Open Windows Settings\n    }",
        "failed = { $platform ->\n    [macos] 打开系统设置\n   *[windows] 打开 Windows 设置\n    }"
    );
    for (platform, english, chinese) in [
        ("macos", "Open System Settings", "打开系统设置"),
        ("windows", "Open Windows Settings", "打开 Windows 设置")
    ] {
        let error = PreparedError::new(
            ProtocolError::new("failed", message("failed", [])),
            Some(&resources),
            platform
        )
        .unwrap();
        assert_eq!(error.messages[&SupportedLocale::English], english);
        assert_eq!(error.messages[&SupportedLocale::SimplifiedChinese], chinese);
    }
}

#[test]
fn external_literals_and_single_language_packages_have_predictable_fallbacks() {
    let error = ProtocolError::new("external", "Original external failure");
    let prepared = PreparedError::new(error.clone(), None, "windows").unwrap();
    assert!(
        prepared
            .messages
            .values()
            .all(|text| text == "Original external failure")
    );
    assert!(PreparedError::new(error, None, "linux").is_err());
    let resources = ResourceSet::new(
        SupportedLocale::SimplifiedChinese,
        [(
            SupportedLocale::SimplifiedChinese,
            vec![("zh.ftl".into(), "failed = 失败".into())]
        )]
        .into(),
        false
    )
    .unwrap();
    let prepared = PreparedError::new(
        ProtocolError::new("failed", message("failed", [])),
        Some(&resources),
        "windows"
    )
    .unwrap();
    assert!(prepared.messages.values().all(|text| text == "失败"));
}

#[test]
fn shipping_builtin_errors_prepare_both_languages_and_preserve_original_diagnostics() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/extensions/built-in");
    for package in ["application", "calculator", "clipboard", "script", "system"] {
        let resources = _resources(
            &std::fs::read_to_string(root.join(package).join("locales/en.ftl")).unwrap(),
            &std::fs::read_to_string(root.join(package).join("locales/zh-Hans.ftl")).unwrap()
        );
        let prepared = resources.prepared();
        let messages: Vec<_> = prepared.locales[&SupportedLocale::English]
            .contracts
            .keys()
            .filter(|id| id.starts_with("error-"))
            .collect();
        assert!(!messages.is_empty(), "{package}");
        for id in messages {
            for platform in ["macos", "windows"] {
                for outcome in ["cancelled", "rejected", "failed", "unconfirmed"] {
                    let arguments = if id == "error-host-service" {
                        vec![("outcome".into(), outcome.into())]
                    } else {
                        Vec::new()
                    };
                    let original =
                        ProtocolError::new("test_failure", message(id.clone(), arguments))
                            .with_detail("EACCES: /Original/私密.txt");
                    let error = PreparedError::new(original.clone(), Some(&resources), platform)
                        .unwrap_or_else(|error| panic!("{package}: {id}: {platform}: {error}"));
                    assert_eq!(error.error, original);
                    assert_ne!(
                        error.messages[&SupportedLocale::English],
                        error.messages[&SupportedLocale::SimplifiedChinese],
                        "{package}: {id}"
                    );
                    assert!(!format!("{error:?}").contains("私密"));
                }
            }
        }
    }
}

fn _resources(english: &str, chinese: &str) -> ResourceSet {
    ResourceSet::new(
        SupportedLocale::English,
        [
            (
                SupportedLocale::English,
                vec![("en.ftl".into(), english.into())]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![("zh.ftl".into(), chinese.into())]
            )
        ]
        .into(),
        true
    )
    .unwrap()
}

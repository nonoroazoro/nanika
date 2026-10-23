use nanika_localization::{ResourceSet, SupportedLocale, validate_progress_presentation};
use nanika_protocol::{OperationProgress, message};

#[test]
fn progress_checks_both_languages_and_platforms_without_changing_work_units() {
    let progress = OperationProgress {
        label: message("scan", []),
        completed: 2,
        total: Some(5)
    };
    let before = progress.clone();
    for (english, chinese, valid) in [
        ("Scan".to_owned(), "扫描".to_owned(), true),
        ("x".repeat(128), "界".repeat(42), true),
        ("x".repeat(129), "扫描".to_owned(), false),
        ("Scan".to_owned(), "界".repeat(43), false),
        (
            "{ $platform ->\n    [macos] Scan\n   *[windows] { \"\" }\n    }".to_owned(),
            "扫描".to_owned(),
            false
        )
    ] {
        let resources = ResourceSet::new(
            SupportedLocale::English,
            [
                (
                    SupportedLocale::English,
                    vec![("en.ftl".into(), format!("scan = {english}"))]
                ),
                (
                    SupportedLocale::SimplifiedChinese,
                    vec![("zh.ftl".into(), format!("scan = {chinese}"))]
                )
            ]
            .into(),
            true
        )
        .unwrap();
        assert_eq!(
            validate_progress_presentation(&progress, Some(&resources)).is_ok(),
            valid
        );
        let mut missing = progress.clone();
        missing.label = message("missing", []);
        assert!(validate_progress_presentation(&missing, Some(&resources)).is_err());
    }
    assert!(validate_progress_presentation(&progress, None).is_err());
    assert_eq!(progress, before);
}

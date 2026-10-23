use nanika_localization::{ResourceSet, SupportedLocale, validate_action_presentation};
use nanika_protocol::{Action, message};

#[test]
fn every_action_destination_is_checked_in_both_locales_and_platform_branches() {
    let resources = _resources("Run", "运行");
    let action = Action::primary("run", message("label", []));
    validate_action_presentation(std::slice::from_ref(&action), Some(&resources)).unwrap();
    assert!(validate_action_presentation(std::slice::from_ref(&action), None).is_err());
    assert!(
        validate_action_presentation(
            &[Action::primary("run", message("unknown", []))],
            Some(&resources)
        )
        .is_err()
    );
    let oversized = _resources("Run", &"界".repeat(129));
    assert!(validate_action_presentation(std::slice::from_ref(&action), Some(&oversized)).is_err());
    let maximum = _resources("Run", &"界".repeat(128));
    validate_action_presentation(std::slice::from_ref(&action), Some(&maximum)).unwrap();
    let windows = _resources(
        &format!(
            "{{ $platform ->\n    [macos] Run\n   *[windows] {}\n    }}",
            "x".repeat(129)
        ),
        "运行"
    );
    assert!(
        validate_action_presentation(std::slice::from_ref(&action), Some(&windows))
            .unwrap_err()
            .contains("windows")
    );
    let mut confirmation = Action::primary("run", "Run");
    confirmation.allow_default_execution = false;
    confirmation.confirmation_title = Some(message("label", []));
    assert!(
        validate_action_presentation(std::slice::from_ref(&confirmation), Some(&oversized))
            .is_err()
    );
    confirmation.confirmation_title = Some("Run?".into());
    confirmation.confirmation_description = Some(message("label", []));
    validate_action_presentation(std::slice::from_ref(&confirmation), Some(&oversized)).unwrap();
    let description = _resources("Run", &"界".repeat(513));
    assert!(validate_action_presentation(&[confirmation], Some(&description)).is_err());
}

fn _resources(english: &str, chinese: &str) -> ResourceSet {
    ResourceSet::new(
        SupportedLocale::English,
        [
            (
                SupportedLocale::English,
                vec![("en.ftl".into(), format!("label = {english}"))]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![("zh.ftl".into(), format!("label = {chinese}"))]
            )
        ]
        .into(),
        true
    )
    .unwrap()
}

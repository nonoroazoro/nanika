use nanika_localization::{LanguagePreference, LocalizationSnapshot, SupportedLocale};

#[test]
fn preference_changes_have_a_revision_even_when_presentation_does_not_change() {
    let initial = LocalizationSnapshot::new(LanguagePreference::System, Some(&["en-US".into()]));
    let explicit = initial.updated(LanguagePreference::English, None).unwrap();
    assert_eq!(explicit.locale, initial.locale);
    assert_eq!(explicit.revision, initial.revision + 1);
    assert_eq!(
        explicit.updated(LanguagePreference::English, None).unwrap(),
        explicit
    );
    let chinese = explicit
        .updated(LanguagePreference::SimplifiedChinese, None)
        .unwrap();
    assert_eq!(chinese.locale, SupportedLocale::SimplifiedChinese);
    assert_eq!(chinese.revision, explicit.revision + 1);
}

#[test]
fn failed_reads_keep_last_locale_and_explicit_language_ignores_system_changes() {
    let initial = LocalizationSnapshot::new(LanguagePreference::System, None);
    assert_eq!(initial.locale, SupportedLocale::English);
    let chinese = initial
        .updated(
            LanguagePreference::System,
            Some(&["ja-JP".into(), "zh-CN".into()])
        )
        .unwrap();
    assert_eq!(chinese.locale, SupportedLocale::SimplifiedChinese);
    assert_eq!(
        chinese.updated(LanguagePreference::System, None).unwrap(),
        chinese
    );
    let explicit = chinese
        .updated(LanguagePreference::English, Some(&["zh-CN".into()]))
        .unwrap();
    assert_eq!(explicit.locale, SupportedLocale::English);
    assert_eq!(
        explicit
            .updated(LanguagePreference::English, Some(&["zh-CN".into()]))
            .unwrap(),
        explicit
    );
    let unsupported = chinese
        .updated(LanguagePreference::System, Some(&["zh-Hant".into()]))
        .unwrap();
    assert_eq!(unsupported.locale, SupportedLocale::English);
}

#[test]
fn revisions_cannot_exceed_the_wire_integer_range() {
    let current = LocalizationSnapshot {
        revision: nanika_protocol::MAX_MESSAGE_INTEGER as u64,
        ..Default::default()
    };
    assert!(
        current
            .updated(LanguagePreference::SimplifiedChinese, None)
            .is_err()
    );
    assert_eq!(
        current.updated(LanguagePreference::System, None).unwrap(),
        current
    );
}

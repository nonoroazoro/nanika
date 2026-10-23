use nanika_localization::{LanguagePreference, SupportedLocale};

#[test]
fn ordered_language_negotiation_respects_explicit_scripts() {
    for (languages, expected) in [
        (vec!["en-US", "zh-CN"], SupportedLocale::English),
        (vec!["en-GB"], SupportedLocale::English),
        (
            vec!["ja-JP", "zh-CN", "en-US"],
            SupportedLocale::SimplifiedChinese
        ),
        (
            vec!["zh-Hant", "zh-TW", "zh-HK", "en-US"],
            SupportedLocale::English
        ),
        (vec!["zh-Hant-CN", "en"], SupportedLocale::English),
        (vec!["zh-Hans-TW"], SupportedLocale::SimplifiedChinese),
        (vec!["ZH-hANS-cn"], SupportedLocale::SimplifiedChinese),
        (vec!["zh-SG"], SupportedLocale::SimplifiedChinese),
        (vec!["zh"], SupportedLocale::SimplifiedChinese),
        (
            vec!["unknown", "", "zh-Hans"],
            SupportedLocale::SimplifiedChinese
        ),
        (vec!["ja-JP"], SupportedLocale::English),
        (vec![], SupportedLocale::English)
    ] {
        assert_eq!(SupportedLocale::negotiate(languages), expected);
    }
}

#[test]
fn failed_refresh_keeps_last_valid_locale_and_explicit_preference_wins() {
    let chinese = SupportedLocale::SimplifiedChinese;
    let english = SupportedLocale::English;
    assert_eq!(LanguagePreference::System.resolve(None, None), english);
    assert_eq!(
        LanguagePreference::System.resolve(None, Some(chinese)),
        chinese
    );
    assert_eq!(
        LanguagePreference::System.resolve(Some(&["ja-JP".into()]), Some(chinese)),
        english
    );
    assert_eq!(
        LanguagePreference::English.resolve(Some(&["zh".into()]), Some(chinese)),
        english
    );
    assert_eq!(
        LanguagePreference::SimplifiedChinese.resolve(None, Some(english)),
        chinese
    );
    assert!(serde_json::from_str::<LanguagePreference>("\"en-US\"").is_err());
    assert!(serde_json::from_str::<SupportedLocale>("\"system\"").is_err());
}

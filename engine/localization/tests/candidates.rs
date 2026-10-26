use nanika_localization::{ResourceSet, SupportedLocale, candidate_search_text};
use nanika_protocol::{Action, Candidate, CandidateKind, CandidateSubtitle, message};

#[test]
fn bilingual_search_data_is_prepared_without_rewriting_the_source_or_indexing_subtitles() {
    let resources = _resources(
        "title = { $platform ->\n    [windows] Empty Recycle Bin\n   *[macos] Empty Trash\n    }\nalias = delete\nsubtitle = System",
        "title = { $platform ->\n    [windows] 清空回收站\n   *[macos] 清倒废纸篓\n    }\nalias = 删除\nsubtitle = 系统"
    );
    let candidate = _candidate();
    let before = candidate.clone();
    assert_eq!(
        candidate_search_text(&candidate, Some(&resources), "macos").unwrap(),
        ["Empty Trash", "清倒废纸篓", "delete", "删除"]
    );
    assert_eq!(
        candidate_search_text(&candidate, Some(&resources), "windows").unwrap(),
        ["Empty Recycle Bin", "清空回收站", "delete", "删除"]
    );
    assert_eq!(candidate, before);
    assert!(candidate_search_text(&candidate, None, "macos").is_err());
    let mut missing = candidate.clone();
    missing.subtitle = Some(CandidateSubtitle::Description(message("unknown", [])));
    assert!(
        candidate_search_text(&missing, Some(&resources), "macos")
            .unwrap_err()
            .contains("subtitle")
    );
    let mut literals = candidate;
    literals.title = "report.pdf".into();
    literals.subtitle = Some(CandidateSubtitle::Description(
        "/Users/example/report.pdf".into()
    ));
    literals.aliases = vec!["invoice".into(), "report.pdf".into(), "".into()];
    assert_eq!(
        candidate_search_text(&literals, None, "macos").unwrap(),
        ["report.pdf", "invoice"]
    );
}

#[test]
fn candidate_admission_checks_inactive_platforms_and_all_rendered_destinations() {
    let valid = "title = Title\nalias = Alias\nsubtitle = Subtitle";
    for field in ["title", "alias", "subtitle"] {
        let invalid = format!(
            "{field} = {{ $platform ->\n    [macos] Valid\n   *[windows] {{ \"\" }}\n    }}"
        );
        let chinese = valid
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{field} =")) {
                    invalid.as_str()
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let resources = _resources(valid, &chinese);
        let error = candidate_search_text(&_candidate(), Some(&resources), "macos").unwrap_err();
        assert!(error.contains(field));
        assert!(error.contains("windows"));
    }
}

#[test]
fn search_uses_each_selected_fallback_bundle_without_losing_partial_translations() {
    for (default_locale, sources, expected) in [
        (
            SupportedLocale::English,
            vec![(
                SupportedLocale::English,
                "title = Title\nalias = Alias\nsubtitle = Subtitle"
            )],
            vec!["Title", "Alias"]
        ),
        (
            SupportedLocale::SimplifiedChinese,
            vec![(
                SupportedLocale::SimplifiedChinese,
                "title = 标题\nalias = 别名\nsubtitle = 描述"
            )],
            vec!["标题", "别名"]
        ),
        (
            SupportedLocale::English,
            vec![
                (
                    SupportedLocale::English,
                    "title = Title\nalias = Alias\nsubtitle = Subtitle"
                ),
                (SupportedLocale::SimplifiedChinese, "alias = 别名"),
            ],
            vec!["Title", "Alias", "别名"]
        )
    ] {
        let resources = ResourceSet::new(
            default_locale,
            sources
                .into_iter()
                .map(|(locale, source)| (locale, vec![("fixture.ftl".into(), source.into())]))
                .collect(),
            false
        )
        .unwrap();
        for platform in ["macos", "windows"] {
            assert_eq!(
                candidate_search_text(&_candidate(), Some(&resources), platform).unwrap(),
                expected
            );
        }
    }
}

#[test]
fn fallback_and_transitive_platform_dependencies_still_validate_inactive_output() {
    let resources = ResourceSet::new(
        SupportedLocale::English,
        [(SupportedLocale::English, vec![("en.ftl".into(),
            "title = { native-title }\nnative-title = { $platform ->\n    [macos] Safe title\n   *[windows] { \"\" }\n    }\nalias = Alias\nsubtitle = Subtitle".into()
        )])].into(),
        false
    ).unwrap();
    let error = candidate_search_text(&_candidate(), Some(&resources), "macos").unwrap_err();
    assert!(
        error.contains("title") && error.contains("windows"),
        "{error}"
    );
}

fn _candidate() -> Candidate {
    Candidate {
        kind: CandidateKind::Action,
        entry_id: "empty".into(),
        title: message("title", []),
        subtitle: Some(CandidateSubtitle::Label(message("subtitle", []))),
        action_id: "run".into(),
        actions: vec![Action::primary("run", "Run")],
        aliases: vec![message("alias", []), message("alias", [])],
        icon: None
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
                vec![("zh-Hans.ftl".into(), chinese.into())]
            )
        ]
        .into(),
        true
    )
    .unwrap()
}

#[test]
fn supplied_application_names_are_indexed_together_without_package_resources() {
    let mut candidate = _candidate();
    candidate.title = nanika_protocol::LocalizedText::Translated(nanika_protocol::TranslatedText {
        fallback: "NetEaseMusic".into(),
        translations: [
            ("en".into(), "NetEaseMusic".into()),
            ("zh-Hans".into(), "网易云音乐".into())
        ]
        .into()
    });
    candidate.aliases.clear();
    candidate.subtitle = None;
    assert_eq!(
        candidate_search_text(&candidate, None, "macos").unwrap(),
        ["NetEaseMusic", "网易云音乐"]
    );
    for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
        let expected = if locale == SupportedLocale::English {
            "NetEaseMusic"
        } else {
            "网易云音乐"
        };
        assert_eq!(
            nanika_localization::format_text(&candidate.title, None, locale, "macos", 128, false)
                .unwrap(),
            expected
        );
    }
    let resources = ResourceSet::new(
        SupportedLocale::English,
        [(
            SupportedLocale::English,
            vec![("en.ftl".into(), "other = Other".into())]
        )]
        .into(),
        false
    )
    .unwrap();
    assert_eq!(
        resources
            .format(
                SupportedLocale::SimplifiedChinese,
                &candidate.title,
                "macos",
                128,
                false
            )
            .unwrap(),
        "网易云音乐"
    );
    let nanika_protocol::LocalizedText::Translated(value) = &mut candidate.title else {
        unreachable!()
    };
    value.translations.insert("ja".into(), "bad\0".into());
    assert!(candidate_search_text(&candidate, None, "macos").is_err());
}

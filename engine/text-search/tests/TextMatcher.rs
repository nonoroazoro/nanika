use crate::{TextMatcher, TextQuery, normalize_query};

#[test]
fn normalization_collapses_punctuation_case_and_whitespace() {
    assert_eq!(normalize_query("  Git-Hub   DESKTOP "), "git hub desktop");
}

#[test]
fn exact_prefix_word_prefix_and_fuzzy_share_the_app_list_tiers() {
    let mut matcher = TextMatcher::new();
    for (query, title, tier) in [
        ("cal", "Cal", 3),
        ("cal", "Calculator", 2),
        ("cal", "Open Calculator", 1),
        ("clc", "Calculator", 0)
    ] {
        let matched = matcher
            .score(&TextQuery::new(query), &[normalize_query(title)])
            .unwrap();
        assert_eq!(matched.tier, tier, "{query}: {title}");
    }
}

#[test]
fn weak_matches_are_rejected_and_every_cross_field_term_is_required() {
    let mut matcher = TextMatcher::new();
    let weak = format!("a{}b{}c", "x".repeat(200), "y".repeat(200));
    assert!(matcher.score(&TextQuery::new("abc"), &[weak]).is_none());
    let values = vec![normalize_query("Folders"), normalize_query("Applications")];
    assert_eq!(
        matcher
            .score(&TextQuery::new("app folder"), &values)
            .unwrap()
            .tier,
        1
    );
    assert_eq!(
        matcher
            .score(&TextQuery::new("folder app"), &values)
            .unwrap()
            .tier,
        1
    );
    assert!(
        matcher
            .score(&TextQuery::new("app missing"), &values)
            .is_none()
    );
}

#[test]
fn unicode_matching_does_not_implicitly_add_pinyin() {
    let mut matcher = TextMatcher::new();
    let values = vec![normalize_query("音乐"), normalize_query("Music")];
    assert!(
        matcher
            .score(&TextQuery::new("音乐music"), &values)
            .is_some()
    );
    assert!(matcher.score(&TextQuery::new("yinyue"), &values).is_none());
    assert!(matcher.score(&TextQuery::new("yy"), &values).is_none());
    assert!(
        matcher
            .score(&TextQuery::new("résumé"), &["r".into(), "ésumé".into()])
            .is_none()
    );
}

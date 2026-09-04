use nanika_search::{InputHistory, normalize_history_key};

#[test]
fn history_preserves_draft_and_deduplicates_case() {
    let mut history = InputHistory::new(2);
    history.record("one");
    history.record("two");
    history.record("ONE");
    assert_eq!(history.entries(), &["two", "ONE"]);
    assert_eq!(history.older("draft"), Some("ONE".to_owned()));
    assert_eq!(history.older("ignored"), Some("two".to_owned()));
    assert_eq!(history.newer(), Some("ONE".to_owned()));
    assert_eq!(history.newer(), Some("draft".to_owned()));
}

#[test]
fn history_identity_preserves_command_punctuation() {
    let mut history = InputHistory::new(10);
    history.record("git --help");
    history.record("git help");
    history.record("C++");
    history.record("C#");
    assert_eq!(history.entries().len(), 4);
    assert_ne!(
        normalize_history_key("git --help"),
        normalize_history_key("git help")
    );
    assert_ne!(normalize_history_key("C++"), normalize_history_key("C#"));
}

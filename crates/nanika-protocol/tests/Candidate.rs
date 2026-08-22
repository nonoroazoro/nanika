use nanika_protocol::Candidate;

#[test]
fn aliases_default_for_older_frames() {
    let candidate: Candidate =
        serde_json::from_str(r#"{"entry_id":"entry","title":"Title","action_id":"open"}"#)
            .expect("candidate should decode");
    assert!(candidate.aliases.is_empty());
}

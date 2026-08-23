use nanika_protocol::Candidate;

#[test]
fn aliases_are_required_by_protocol_v1() {
    let candidate = serde_json::from_str::<Candidate>(
        r#"{"entry_id":"entry","title":"Title","action_id":"open"}"#,
    );
    assert!(candidate.is_err());
}

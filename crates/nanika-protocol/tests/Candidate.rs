use nanika_protocol::{Candidate, IconReference};

#[test]
fn aliases_are_required_by_protocol_v1() {
    let candidate = serde_json::from_str::<Candidate>(
        r#"{"entry_id":"entry","title":"Title","action_id":"open","icon":null}"#,
    );
    assert!(candidate.is_err());
}

#[test]
fn icon_references_reject_paths() {
    assert!(IconReference::new("application-fallback-v1").is_ok());
    assert!(IconReference::new("../outside").is_err());
    assert!(IconReference::new("folder/icon.png").is_err());
}

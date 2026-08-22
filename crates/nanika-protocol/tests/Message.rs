use nanika_protocol::Message;

#[test]
fn snapshot_completion_defaults_for_older_frames() {
    let message: Message = serde_json::from_str(
        r#"{"type":"snapshot","request_id":"query","generation":1,"entries":[]}"#,
    )
    .expect("snapshot should decode");
    assert!(matches!(message, Message::Snapshot { complete: true, .. }));
}

#[test]
fn invocation_identifies_the_selected_entry_and_action() {
    let message = Message::Invoke {
        request_id: "invoke".to_owned(),
        generation: 7,
        entry_id: "application.firefox".to_owned(),
        action_id: "application.open".to_owned(),
    };
    let encoded = serde_json::to_value(message).expect("invoke should encode");
    assert_eq!(encoded["entry_id"], "application.firefox");
    assert_eq!(encoded["action_id"], "application.open");
}

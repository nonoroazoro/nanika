use nanika_protocol::{
    HostServiceRequest, LaunchArguments, LaunchDescriptor, Message, SettingControl, SettingField,
    SettingValue, SettingsContribution,
};

#[test]
fn snapshot_completion_is_required_by_protocol_v1() {
    let message = serde_json::from_str::<Message>(
        r#"{"type":"snapshot","request_id":"query","generation":1,"entries":[]}"#,
    );
    assert!(message.is_err());
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

#[test]
fn refresh_completion_preserves_request_identity_and_generation() {
    let message = Message::Refreshed {
        request_id: "refresh".to_owned(),
        generation: 11,
    };
    let encoded = serde_json::to_value(message).expect("refresh should encode");
    assert_eq!(encoded["type"], "refreshed");
    assert_eq!(encoded["request_id"], "refresh");
    assert_eq!(encoded["generation"], 11);
}

#[test]
fn host_requests_are_bound_to_the_parent_invocation() {
    let message = Message::HostRequest {
        request_id: "service-1".to_owned(),
        parent_request_id: "invoke-1".to_owned(),
        generation: 5,
        request: HostServiceRequest::Launch {
            descriptor: LaunchDescriptor::Program {
                program: "tool".to_owned(),
                arguments: LaunchArguments::Structured {
                    values: vec!["--help".to_owned()],
                },
                working_directory: None,
            },
        },
    };
    let encoded = serde_json::to_value(message).expect("host request should encode");
    assert_eq!(encoded["type"], "hostRequest");
    assert_eq!(encoded["parent_request_id"], "invoke-1");
    assert_eq!(encoded["request"]["service"], "launch");
}

#[test]
fn settings_contributions_are_typed_and_bounded() {
    let contribution = SettingsContribution {
        title: "Test".to_owned(),
        fields: vec![SettingField {
            key: "enabled".to_owned(),
            title: "Enabled".to_owned(),
            description: None,
            control: SettingControl::Toggle,
            value: SettingValue::Boolean { value: true },
        }],
    };
    contribution.validate().expect("settings should validate");
    let message = Message::Settings {
        request_id: "settings".to_owned(),
        contribution,
    };
    let encoded = serde_json::to_value(message).expect("settings should encode");
    assert_eq!(encoded["type"], "settings");
    assert_eq!(
        encoded["contribution"]["fields"][0]["value"]["kind"],
        "boolean"
    );
}

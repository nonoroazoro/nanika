use nanika_protocol::{
    DetailView, HostServiceRequest, LaunchArguments, LaunchDescriptor, ListItem, ListLayout,
    ListSection, ListView, Message, NavigationEffect, SettingControl, SettingField, SettingValue,
    SettingsContribution, View, ViewAction, ViewActionStyle,
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
fn pushed_views_are_bounded_host_rendered_documents() {
    let view = View::List {
        list: Box::new(ListView {
            title: "Clipboard History".to_owned(),
            search_placeholder: "Filter entries".to_owned(),
            search_text: String::new(),
            layout: ListLayout::Split,
            sections: vec![ListSection {
                id: "recent".to_owned(),
                title: Some("Recent".to_owned()),
                items: vec![ListItem {
                    id: "entry-1".to_owned(),
                    title: "Example".to_owned(),
                    subtitle: Some("Text".to_owned()),
                    actions: vec![ViewAction {
                        id: "paste".to_owned(),
                        title: "Paste".to_owned(),
                        style: ViewActionStyle::Primary,
                    }],
                }],
            }],
            selected_item_id: Some("entry-1".to_owned()),
            detail: Some(DetailView {
                title: Some("Example".to_owned()),
                body: "Content".to_owned(),
                metadata: Vec::new(),
                actions: Vec::new(),
            }),
            filter: None,
            next_cursor: None,
        }),
    };
    let effect = NavigationEffect::Push {
        view_id: "clipboard.history".to_owned(),
        revision: 1,
        view: Box::new(view),
    };
    effect.validate().expect("view should validate");
    let encoded = serde_json::to_value(Message::Result {
        request_id: "invoke".to_owned(),
        generation: 7,
        effect,
    })
    .expect("view result should encode");
    assert_eq!(encoded["effect"]["kind"], "push");
    assert_eq!(encoded["effect"]["view"]["kind"], "list");
}

#[test]
fn view_validation_rejects_an_unbounded_list() {
    let items = (0..501)
        .map(|index| ListItem {
            id: format!("entry-{index}"),
            title: "Entry".to_owned(),
            subtitle: None,
            actions: Vec::new(),
        })
        .collect();
    let view = View::List {
        list: Box::new(ListView {
            title: "Large".to_owned(),
            search_placeholder: String::new(),
            search_text: String::new(),
            layout: ListLayout::Plain,
            sections: vec![ListSection {
                id: "all".to_owned(),
                title: None,
                items,
            }],
            selected_item_id: None,
            detail: None,
            filter: None,
            next_cursor: None,
        }),
    };
    assert_eq!(
        view.validate().expect_err("large view must fail"),
        "view has too many list items"
    );
}

#[test]
fn list_detail_actions_must_belong_to_the_selected_item() {
    let view = View::List {
        list: Box::new(ListView {
            title: "Examples".to_owned(),
            search_placeholder: String::new(),
            search_text: String::new(),
            layout: ListLayout::Split,
            sections: vec![ListSection {
                id: "all".to_owned(),
                title: None,
                items: vec![ListItem {
                    id: "entry-1".to_owned(),
                    title: "Example".to_owned(),
                    subtitle: None,
                    actions: Vec::new(),
                }],
            }],
            selected_item_id: Some("entry-1".to_owned()),
            detail: Some(DetailView {
                title: None,
                body: "Example".to_owned(),
                metadata: Vec::new(),
                actions: vec![ViewAction {
                    id: "example.open".to_owned(),
                    title: "Open".to_owned(),
                    style: ViewActionStyle::Primary,
                }],
            }),
            filter: None,
            next_cursor: None,
        }),
    };

    assert_eq!(
        view.validate()
            .expect_err("nested detail actions must fail"),
        "list detail actions must be declared on the selected list item"
    );
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

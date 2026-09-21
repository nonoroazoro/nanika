use nanika_protocol::{
    DetailContent, DetailView, ExtensionConfiguration, HostServiceRequest, HostServiceResponse,
    ImageSource, LaunchArguments, LaunchDescriptor, ListItem, ListLayout, ListSection, ListView,
    Message, NavigationEffect, View, ViewAction, ViewActionStyle, ViewItemIcon,
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
        entry_id: "application.example".to_owned(),
        action_id: "application.open".to_owned(),
    };
    let encoded = serde_json::to_value(message).expect("invoke should encode");
    assert_eq!(encoded["entry_id"], "application.example");
    assert_eq!(encoded["action_id"], "application.open");
}

#[test]
fn clipboard_write_response_carries_the_opaque_native_revision() {
    let response = HostServiceResponse::ClipboardWritten { revision: 42 };
    let encoded = serde_json::to_value(response).expect("clipboard response should encode");
    assert_eq!(encoded["service"], "clipboardWritten");
    assert_eq!(encoded["revision"], 42);
}

#[test]
fn resumed_view_events_have_a_platform_neutral_wire_shape() {
    let message = Message::ViewEvent {
        request_id: "resume".to_owned(),
        generation: 7,
        view_id: "clipboard.history".to_owned(),
        revision: 3,
        event: nanika_protocol::ViewEvent::Resumed,
    };
    let encoded = serde_json::to_value(message).expect("view resume should encode");
    assert_eq!(encoded["event"]["kind"], "resumed");
}

#[test]
fn visible_entry_preparation_is_a_requestless_bounded_hint() {
    let message = Message::PrepareEntries {
        generation: 9,
        entry_ids: vec![
            "application.first".to_owned(),
            "application.second".to_owned(),
        ],
    };
    let encoded = serde_json::to_value(message).expect("entry hint should encode");
    assert_eq!(encoded["type"], "prepareEntries");
    assert_eq!(encoded["generation"], 9);
    assert_eq!(encoded["entry_ids"].as_array().expect("entry IDs").len(), 2);
    assert!(encoded.get("request_id").is_none());
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
                    icon: Some(ViewItemIcon::Text),
                    actions: vec![ViewAction {
                        id: "paste".to_owned(),
                        title: "Paste".to_owned(),
                        confirmation_title: None,
                        style: ViewActionStyle::Primary,
                    }],
                }],
            }],
            selected_item_id: Some("entry-1".to_owned()),
            detail: Some(DetailView {
                title: Some("Example".to_owned()),
                content: DetailContent::Text {
                    value: "Content".to_owned(),
                },
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
    assert_eq!(
        encoded["effect"]["view"]["list"]["sections"][0]["items"][0]["icon"],
        "text"
    );
}

#[test]
fn detail_files_must_not_be_empty() {
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Files { files: Vec::new() },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };

    assert_eq!(
        view.validate().expect_err("empty file detail must fail"),
        "detail file count is invalid"
    );
}

#[test]
fn view_action_confirmation_titles_are_validated() {
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                value: "Example".to_owned(),
            },
            metadata: Vec::new(),
            actions: vec![ViewAction {
                id: "example.clear".to_owned(),
                title: "Clear".to_owned(),
                confirmation_title: Some(" ".to_owned()),
                style: ViewActionStyle::Destructive,
            }],
        },
    };

    assert_eq!(
        view.validate()
            .expect_err("blank confirmation title must fail"),
        "view action confirmation title is invalid"
    );
}

#[test]
fn view_action_confirmation_is_limited_to_destructive_actions() {
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                value: "Example".to_owned(),
            },
            metadata: Vec::new(),
            actions: vec![ViewAction {
                id: "example.open".to_owned(),
                title: "Open".to_owned(),
                confirmation_title: Some("Open now".to_owned()),
                style: ViewActionStyle::Primary,
            }],
        },
    };

    assert_eq!(
        view.validate()
            .expect_err("non-destructive confirmation must fail"),
        "view action confirmation requires destructive style"
    );
}

#[test]
fn detail_resource_images_use_relative_png_paths() {
    let resource_path = format!("{}.png", "0123456789abcdef".repeat(4));
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Image {
                source: ImageSource::Resource {
                    path: resource_path,
                },
                alternative_text: "Image".to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };
    view.validate()
        .expect("relative resource image should validate");

    let invalid = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Image {
                source: ImageSource::Resource {
                    path: "../outside.png".to_owned(),
                },
                alternative_text: "Image".to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };
    assert!(invalid.validate().is_err());

    let mutable_name = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Image {
                source: ImageSource::Resource {
                    path: "preview.png".to_owned(),
                },
                alternative_text: "Image".to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };
    assert!(mutable_name.validate().is_err());
}

#[test]
fn view_validation_rejects_an_unbounded_list() {
    let items = (0..501)
        .map(|index| ListItem {
            id: format!("entry-{index}"),
            title: "Entry".to_owned(),
            subtitle: None,
            icon: None,
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
                    icon: None,
                    actions: Vec::new(),
                }],
            }],
            selected_item_id: Some("entry-1".to_owned()),
            detail: Some(DetailView {
                title: None,
                content: DetailContent::Text {
                    value: "Example".to_owned(),
                },
                metadata: Vec::new(),
                actions: vec![ViewAction {
                    id: "example.open".to_owned(),
                    title: "Open".to_owned(),
                    confirmation_title: None,
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
fn configuration_updates_carry_a_complete_snapshot() {
    let message = Message::ConfigurationChanged {
        request_id: "configuration".to_owned(),
        configuration: ExtensionConfiguration::new(std::collections::BTreeMap::from([(
            "example.enabled".to_owned(),
            serde_json::json!(true),
        )])),
    };
    let encoded = serde_json::to_value(message).expect("configuration should encode");
    assert_eq!(encoded["type"], "configurationChanged");
    assert_eq!(encoded["configuration"]["example.enabled"], true);
}

#[test]
fn native_view_icons_are_opaque_and_validated() {
    let reference = nanika_protocol::IconReference::new("file-icon").expect("reference");
    assert_eq!(
        serde_json::to_value(ViewItemIcon::Native(reference.clone())).expect("wire shape"),
        serde_json::json!({"native": {"key": "file-icon"}})
    );
    let mut view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Files {
                files: vec![nanika_protocol::ViewFile {
                    name: "example.pkg".to_owned(),
                    path: "/example/example.pkg".to_owned(),
                    icon: Some(reference),
                }],
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };
    view.validate().expect("valid icon reference");
    let mut oversized = view.clone();
    if let View::Detail { detail } = &mut oversized
        && let DetailContent::Files { files } = &mut detail.content
    {
        files[0].path = "a".repeat(1024 * 1024 + 1);
    }
    assert_eq!(
        oversized.validate().expect_err("oversized paths"),
        "detail file paths exceed the supported size"
    );

    let invalid: nanika_protocol::IconReference =
        serde_json::from_value(serde_json::json!({"key": "../outside"}))
            .expect("untrusted serialized reference");
    if let View::Detail { detail } = &mut view
        && let DetailContent::Files { files } = &mut detail.content
    {
        files[0].icon = Some(invalid.clone());
    }
    assert!(view.validate().is_err());
    let view = View::List {
        list: Box::new(ListView {
            title: "Files".to_owned(),
            search_placeholder: String::new(),
            search_text: String::new(),
            layout: ListLayout::Plain,
            sections: vec![ListSection {
                id: "files".to_owned(),
                title: None,
                items: vec![ListItem {
                    id: "one".to_owned(),
                    title: "example.pkg".to_owned(),
                    subtitle: None,
                    icon: Some(ViewItemIcon::Native(invalid)),
                    actions: Vec::new(),
                }],
            }],
            selected_item_id: None,
            detail: None,
            filter: None,
            next_cursor: None,
        }),
    };
    assert!(view.validate().is_err());
}

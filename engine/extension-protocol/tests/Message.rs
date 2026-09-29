use nanika_protocol::{
    Action, ActionStyle, DetailContent, DetailView, ExtensionConfiguration, HostServiceRequest,
    HostServiceResponse, ImageSource, LaunchArguments, LaunchDescriptor, ListItem, ListLayout,
    ListSection, ListView, Message, NavigationEffect, View, ViewItemIcon,
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
            empty_title: "No items".to_owned(),
            empty_description: "Items will appear here when available.".to_owned(),
            search_text: String::new(),
            layout: ListLayout::Split,
            sections: vec![ListSection {
                id: "recent".to_owned(),
                title: Some("Recent".to_owned()),
                offset: 0,
                total: 1,
                items: vec![ListItem {
                    id: "entry-1".to_owned(),
                    title: "Example".to_owned(),
                    subtitle: Some("Text".to_owned()),
                    icon: Some(ViewItemIcon::Text),
                    actions: vec![Action {
                        icon: None,
                        id: "paste".to_owned(),
                        title: "Paste".to_owned(),
                        confirmation_title: None,
                        allow_default_execution: true,
                        style: ActionStyle::Primary,
                        enabled: true,
                        group: None,
                    }],
                }],
            }],
            collection_id: "test.collection".into(),
            selection: None,
            detail: Some(DetailView {
                title: Some("Example".to_owned()),
                content: DetailContent::Text {
                    text_id: "test.text".into(),
                    chunk_index: 0,
                    total_chunks: 1,
                    value: "Content".to_owned(),
                },
                metadata: Vec::new(),
                actions: Vec::new(),
            }),
            filter: None,
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
                text_id: "test.text".into(),
                chunk_index: 0,
                total_chunks: 1,
                value: "Example".to_owned(),
            },
            metadata: Vec::new(),
            actions: vec![Action {
                icon: None,
                id: "example.clear".to_owned(),
                title: "Clear".to_owned(),
                confirmation_title: Some(" ".to_owned()),
                allow_default_execution: false,
                style: ActionStyle::Destructive,
                enabled: true,
                group: None,
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
                text_id: "test.text".into(),
                chunk_index: 0,
                total_chunks: 1,
                value: "Example".to_owned(),
            },
            metadata: Vec::new(),
            actions: vec![Action {
                icon: None,
                id: "example.open".to_owned(),
                title: "Open".to_owned(),
                confirmation_title: Some("Open now".to_owned()),
                allow_default_execution: false,
                style: ActionStyle::Primary,
                enabled: true,
                group: None,
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
                source: ImageSource {
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
                source: ImageSource {
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
                source: ImageSource {
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
fn delivered_windows_are_bounded_independently_of_collection_size() {
    let items = (0..501)
        .map(|index| ListItem {
            id: format!("entry-{index}"),
            title: "Entry".to_owned(),
            subtitle: None,
            icon: None,
            actions: Vec::new(),
        })
        .collect();
    let mut view = View::List {
        list: Box::new(ListView {
            title: "Large".to_owned(),
            search_placeholder: String::new(),
            empty_title: "No items".to_owned(),
            empty_description: "Items will appear here when available.".to_owned(),
            search_text: String::new(),
            layout: ListLayout::Plain,
            sections: vec![ListSection {
                id: "all".to_owned(),
                title: None,
                offset: 0,
                total: 100_000,
                items,
            }],
            collection_id: "test.collection".into(),
            selection: None,
            detail: None,
            filter: None,
        }),
    };
    assert!(view.validate().is_err());
    let View::List { list } = &mut view else {
        unreachable!()
    };
    list.sections[0].items.truncate(50);
    view.validate().unwrap();
    let View::List { list } = &mut view else {
        unreachable!()
    };
    list.sections[0].offset = 99_980;
    assert!(view.validate().is_err());
}

#[test]
fn list_detail_actions_must_belong_to_the_selected_item() {
    let view = View::List {
        list: Box::new(ListView {
            title: "Examples".to_owned(),
            search_placeholder: String::new(),
            empty_title: "No items".to_owned(),
            empty_description: "Items will appear here when available.".to_owned(),
            search_text: String::new(),
            layout: ListLayout::Split,
            sections: vec![ListSection {
                id: "all".to_owned(),
                title: None,
                offset: 0,
                total: 1,
                items: vec![ListItem {
                    id: "entry-1".to_owned(),
                    title: "Example".to_owned(),
                    subtitle: None,
                    icon: None,
                    actions: Vec::new(),
                }],
            }],
            collection_id: "test.collection".into(),
            selection: None,
            detail: Some(DetailView {
                title: None,
                content: DetailContent::Text {
                    text_id: "test.text".into(),
                    chunk_index: 0,
                    total_chunks: 1,
                    value: "Example".to_owned(),
                },
                metadata: Vec::new(),
                actions: vec![Action {
                    icon: None,
                    id: "example.open".to_owned(),
                    title: "Open".to_owned(),
                    confirmation_title: None,
                    allow_default_execution: true,
                    style: ActionStyle::Primary,
                    enabled: true,
                    group: None,
                }],
            }),
            filter: None,
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
            empty_title: "No items".to_owned(),
            empty_description: "Items will appear here when available.".to_owned(),
            search_text: String::new(),
            layout: ListLayout::Plain,
            sections: vec![ListSection {
                id: "files".to_owned(),
                title: None,
                offset: 0,
                total: 1,
                items: vec![ListItem {
                    id: "one".to_owned(),
                    title: "example.pkg".to_owned(),
                    subtitle: None,
                    icon: Some(ViewItemIcon::Native(invalid)),
                    actions: Vec::new(),
                }],
            }],
            collection_id: "test.collection".into(),
            selection: None,
            detail: None,
            filter: None,
        }),
    };
    assert!(view.validate().is_err());
}

#[test]
fn configuration_progress_requires_real_bounded_work_units() {
    use nanika_protocol::OperationProgress;
    for progress in [
        OperationProgress {
            label: "Scanning".into(),
            completed: 1,
            total: Some(2),
        },
        OperationProgress {
            label: "Connecting".into(),
            completed: 0,
            total: None,
        },
    ] {
        assert!(progress.validate().is_ok());
    }
    for progress in [
        OperationProgress {
            label: "".into(),
            completed: 0,
            total: None,
        },
        OperationProgress {
            label: "x".repeat(129),
            completed: 0,
            total: None,
        },
        OperationProgress {
            label: "Scanning".into(),
            completed: 3,
            total: Some(2),
        },
        OperationProgress {
            label: "Scanning".into(),
            completed: 0,
            total: Some(0),
        },
        OperationProgress {
            label: "Connecting".into(),
            completed: 1,
            total: None,
        },
    ] {
        assert!(progress.validate().is_err());
    }
}

#[test]
fn system_service_rejects_unknown_operations_and_round_trips_submission_receipt() {
    use nanika_protocol::SystemAction;
    for action in [
        SystemAction::Lock,
        SystemAction::Sleep,
        SystemAction::TurnOffDisplays,
        SystemAction::LogOut,
        SystemAction::Restart,
        SystemAction::ShutDown,
        SystemAction::OpenTrash,
        SystemAction::EmptyTrash,
    ] {
        let request = HostServiceRequest::SystemAction { action };
        assert_eq!(
            serde_json::from_str::<HostServiceRequest>(&serde_json::to_string(&request).unwrap())
                .unwrap(),
            request
        );
    }
    assert!(
        serde_json::from_str::<HostServiceRequest>(
            r#"{"service":"systemAction","action":"shell"}"#
        )
        .is_err()
    );
    let response = HostServiceResponse::SystemActionSubmitted;
    assert_eq!(
        serde_json::from_str::<HostServiceResponse>(&serde_json::to_string(&response).unwrap())
            .unwrap(),
        response
    );
}

#[test]
fn repeated_section_identities_are_rejected_before_rendering() {
    let view = View::List {
        list: Box::new(ListView {
            title: "Sections".into(),
            search_placeholder: String::new(),
            empty_title: "No items".to_owned(),
            empty_description: "Items will appear here when available.".to_owned(),
            search_text: String::new(),
            layout: ListLayout::Plain,
            sections: vec![
                ListSection {
                    id: "same".into(),
                    title: None,
                    offset: 0,
                    total: 0,
                    items: Vec::new(),
                },
                ListSection {
                    id: "same".into(),
                    title: None,
                    offset: 0,
                    total: 0,
                    items: Vec::new(),
                },
            ],
            collection_id: "test.collection".into(),
            selection: None,
            detail: None,
            filter: None,
        }),
    };
    assert_eq!(
        view.validate().unwrap_err(),
        "view list section ids must be unique"
    );
}

#[test]
fn image_sources_accept_only_immutable_resource_paths() {
    assert!(
        serde_json::from_str::<ImageSource>(
            r#"{"kind":"dataUrl","value":"data:image/png;base64,AA=="}"#
        )
        .is_err()
    );
    let source: ImageSource =
        serde_json::from_value(serde_json::json!({"path": format!("{}.png", "a".repeat(64))}))
            .unwrap();
    assert!(nanika_protocol::is_valid_resource_path(&source.path));
}

#[test]
fn view_changes_are_instance_scoped_and_bounded_text_chunks_preserve_controls() {
    assert_eq!(
        serde_json::to_value(Message::ViewsChanged).unwrap(),
        serde_json::json!({"type":"viewsChanged"})
    );
    let mut view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                value: "\0\u{b}\n原文".into(),
                text_id: "test.text".into(),
                chunk_index: 0,
                total_chunks: 1,
            },
            metadata: vec![],
            actions: vec![],
        },
    };
    view.validate().unwrap();
    let View::Detail { detail } = &mut view else {
        unreachable!()
    };
    detail.content = DetailContent::Text {
        value: "x".repeat(16_385),
        text_id: "test.text".into(),
        chunk_index: 0,
        total_chunks: 1,
    };
    assert!(view.validate().is_err());
}

#[test]
fn text_preview_resident_budget_bounds_invisible_and_maximum_size_chunks() {
    for (chunks, index, valid) in [
        (1, 0, true),
        (nanika_protocol::MAX_DETAIL_TEXT_CHUNKS, 0, true),
        (
            nanika_protocol::MAX_DETAIL_TEXT_CHUNKS,
            nanika_protocol::MAX_DETAIL_TEXT_CHUNKS - 1,
            true,
        ),
        (nanika_protocol::MAX_DETAIL_TEXT_CHUNKS + 1, 0, false),
        (u32::MAX as usize, 0, false),
    ] {
        let view = View::Detail {
            detail: DetailView {
                title: None,
                content: DetailContent::Text {
                    value: "\u{200b}".repeat(nanika_protocol::DETAIL_TEXT_BATCH_CHARS),
                    text_id: "test.bounded".into(),
                    chunk_index: index,
                    total_chunks: chunks,
                },
                metadata: vec![],
                actions: vec![],
            },
        };
        assert_eq!(view.validate().is_ok(), valid);
    }
}

#[test]
fn search_and_filter_require_a_positive_integer_viewport_demand() {
    for kind in ["searchChanged", "filterChanged"] {
        let mut value = serde_json::json!({
            "kind": kind, "text": "image", "filter_id": "contentType", "value": "images"
        });
        assert!(serde_json::from_value::<nanika_protocol::ViewEvent>(value.clone()).is_err());
        for demand in [
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!(4294967296_u64),
        ] {
            value["minimum_items"] = demand;
            assert!(serde_json::from_value::<nanika_protocol::ViewEvent>(value.clone()).is_err());
        }
        value["minimum_items"] = serde_json::json!(27);
        let event = serde_json::from_value::<nanika_protocol::ViewEvent>(value).unwrap();
        assert_eq!(serde_json::to_value(event).unwrap()["minimum_items"], 27);
    }
}

#[test]
fn list_empty_copy_is_bounded_before_rendering() {
    let mut view = View::List {
        list: Box::new(ListView {
            title: "History".into(),
            search_placeholder: String::new(),
            search_text: String::new(),
            empty_title: "No entries".into(),
            empty_description: String::new(),
            layout: ListLayout::Plain,
            sections: Vec::new(),
            collection_id: "test.collection".into(),
            selection: None,
            detail: None,
            filter: None,
        }),
    };
    view.validate().unwrap();
    let encoded = serde_json::to_string(&view).unwrap();
    assert_eq!(serde_json::from_str::<View>(&encoded).unwrap(), view);
    for (title, description, valid) in [
        ("x".repeat(256), "x".repeat(512), true),
        ("x".repeat(257), String::new(), false),
        ("No entries".into(), "x".repeat(513), false),
        (" ".into(), String::new(), false),
        ("Invalid\0".into(), String::new(), false),
        ("No entries".into(), "Invalid\0".into(), false),
    ] {
        let View::List { list } = &mut view else {
            unreachable!()
        };
        list.empty_title = title;
        list.empty_description = description;
        assert_eq!(view.validate().is_ok(), valid);
    }
}

#[test]
fn content_reads_require_matching_identity_and_complete_requested_window() {
    let mut view = View::List {
        list: Box::new(ListView {
            title: "Collection".into(),
            search_placeholder: "Search".into(),
            search_text: String::new(),
            empty_title: "Empty".into(),
            empty_description: String::new(),
            layout: ListLayout::Plain,
            collection_id: "collection".into(),
            selection: None,
            detail: None,
            filter: None,
            sections: vec![ListSection {
                id: "all".into(),
                title: None,
                offset: 10,
                total: 100,
                items: (10..20)
                    .map(|i| ListItem {
                        id: i.to_string(),
                        title: i.to_string(),
                        subtitle: None,
                        icon: None,
                        actions: vec![],
                    })
                    .collect(),
            }],
        }),
    };
    let event = nanika_protocol::ViewEvent::ListRangeChanged {
        collection_id: "collection".into(),
        offset: 10,
        count: std::num::NonZeroU32::new(10).unwrap(),
    };
    view.validate().unwrap();
    event.validate_response(Some(&view)).unwrap();
    assert!(event.validate_response(None).is_err());
    let View::List { list } = &mut view else {
        unreachable!()
    };
    list.sections[0].items.pop();
    assert!(event.validate_response(Some(&view)).is_err());
    let View::List { list } = &mut view else {
        unreachable!()
    };
    list.collection_id = "other".into();
    assert!(event.validate_response(Some(&view)).is_err());
    let text = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                value: "hello".into(),
                text_id: "doc".into(),
                chunk_index: 1,
                total_chunks: 3,
            },
            metadata: vec![],
            actions: vec![],
        },
    };
    let read = nanika_protocol::ViewEvent::TextChunkRequested {
        text_id: "doc".into(),
        index: 1,
    };
    read.validate_response(Some(&text)).unwrap();
    assert!(
        nanika_protocol::ViewEvent::TextChunkRequested {
            text_id: "doc".into(),
            index: 2
        }
        .validate_response(Some(&text))
        .is_err()
    );
    assert!(read.validate_response(None).is_err());
}

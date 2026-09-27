use nanika_protocol::{DetailContent, DetailView, View, ViewEvent};

use crate::authorize_view_event;

#[test]
fn disabled_or_unknown_actions_cannot_be_invoked() {
    let mut disabled = nanika_protocol::Action::primary("disabled", "Disabled");
    disabled.enabled = false;
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                pagination: None,
                value: "content".to_owned(),
            },
            metadata: Vec::new(),
            actions: vec![disabled, nanika_protocol::Action::primary("open", "Open")],
        },
    };
    for id in ["disabled", "unknown"] {
        assert!(
            authorize_view_event(
                &view,
                &ViewEvent::ActionInvoked {
                    invocation: nanika_protocol::ActionInvocation::Default,
                    item_id: None,
                    action_id: id.to_owned(),
                }
            )
            .is_err()
        );
    }
    assert!(
        authorize_view_event(
            &view,
            &ViewEvent::ActionInvoked {
                invocation: nanika_protocol::ActionInvocation::Default,
                item_id: None,
                action_id: "open".to_owned(),
            }
        )
        .is_ok()
    );
    assert!(
        authorize_view_event(
            &view,
            &ViewEvent::ActionInvoked {
                invocation: nanika_protocol::ActionInvocation::Default,
                item_id: Some("another-item".to_owned()),
                action_id: "open".to_owned(),
            }
        )
        .is_err()
    );
}

#[test]
fn resumed_events_are_authorized_for_every_host_rendered_view() {
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                pagination: None,
                value: "content".to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    };

    authorize_view_event(&view, &ViewEvent::Resumed)
        .expect("an active extension view should receive resume lifecycle events");
}

fn text_view(value: &str) -> View {
    View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
                pagination: None,
                value: value.to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    }
}

#[test]
fn completed_view_is_presented() {
    let mut navigation = crate::NavigationState::default();
    let completion = nanika_host::RuntimeInvocationCompletion {
        instance_id: 1,
        outcome: nanika_host::ExtensionInvocationOutcome::Completed {
            effect: nanika_protocol::NavigationEffect::Push {
                view_id: "created-view".to_owned(),
                revision: 1,
                view: Box::new(text_view("created")),
            },
            has_output: false,
        },
    };
    crate::apply_invocation_completion(completion, |effect| {
        navigation.apply("test.extension", 1, 1, effect)
    })
    .unwrap();
    assert_eq!(navigation.stack.last().unwrap().view_id, "created-view");
}

#[test]
fn completed_view_cleanup_failure_is_preserved() {
    let mut cleanup_attempted = false;
    let completion = nanika_host::RuntimeInvocationCompletion {
        instance_id: 1,
        outcome: nanika_host::ExtensionInvocationOutcome::Completed {
            effect: nanika_protocol::NavigationEffect::Push {
                view_id: "retired-view".to_owned(),
                revision: 1,
                view: Box::new(text_view("retired")),
            },
            has_output: false,
        },
    };
    let error = crate::apply_invocation_completion(completion, |effect| {
        assert!(matches!(
            effect,
            nanika_protocol::NavigationEffect::Push { .. }
        ));
        cleanup_attempted = true;
        Err("closing retired view failed: extension disconnected".to_owned())
    })
    .unwrap_err();
    assert!(cleanup_attempted);
    assert!(error.contains("extension disconnected"));
}

#[test]
fn navigation_rejects_overflow_without_discarding_existing_routes() {
    let mut navigation = crate::NavigationState::default();
    for index in 0..crate::MAX_NAVIGATION_DEPTH {
        navigation
            .apply(
                "test.extension",
                1,
                1,
                nanika_protocol::NavigationEffect::Push {
                    view_id: format!("view-{index}"),
                    revision: 1,
                    view: Box::new(text_view("existing")),
                },
            )
            .unwrap();
    }
    let previous = navigation.stack.last().unwrap().route_id;
    assert!(
        navigation
            .apply(
                "test.extension",
                1,
                1,
                nanika_protocol::NavigationEffect::Push {
                    view_id: "overflow".to_owned(),
                    revision: 1,
                    view: Box::new(text_view("new"))
                }
            )
            .unwrap_err()
            .contains("32")
    );
    assert_eq!(navigation.stack.len(), crate::MAX_NAVIGATION_DEPTH);
    assert_eq!(navigation.stack.last().unwrap().route_id, previous);
    navigation
        .apply(
            "test.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Pop,
        )
        .unwrap();
    navigation
        .apply(
            "test.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Push {
                view_id: "replacement".to_owned(),
                revision: 1,
                view: Box::new(text_view("new")),
            },
        )
        .unwrap();
}

#[test]
fn delayed_invalidation_cannot_cross_session_or_owner_boundaries() {
    let mut old = crate::SearchSession::new(1, tauri::ipc::Channel::new(|_| Ok(())));
    old.navigation
        .apply(
            "old.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Push {
                view_id: "view".to_owned(),
                revision: 1,
                view: Box::new(text_view("old")),
            },
        )
        .unwrap();
    let route = old.navigation.stack[0].clone();
    let mut new = crate::SearchSession::new(2, tauri::ipc::Channel::new(|_| Ok(())));
    new.navigation
        .apply(
            "new.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Push {
                view_id: "view".to_owned(),
                revision: 1,
                view: Box::new(text_view("new")),
            },
        )
        .unwrap();
    let original = new.navigation.stack[0].view.clone();
    let old_guard = old.view_operation_lock.lock().unwrap();
    assert!(new.view_operation_lock.try_lock().is_ok());
    let mut state = crate::DesktopRuntime {
        session: Some(new),
        ..Default::default()
    };
    crate::view_invalidation_delivery::apply_completion(
        &mut state,
        1,
        &route,
        2,
        text_view("stale"),
    );
    crate::view_invalidation_delivery::apply_completion(
        &mut state,
        2,
        &route,
        2,
        text_view("wrong owner"),
    );
    let current = &state.session.as_ref().unwrap().navigation.stack[0];
    assert!(std::sync::Arc::ptr_eq(&original, &current.view));
    assert_eq!(current.revision, 1);
    let current = current.clone();
    crate::view_invalidation_delivery::apply_completion(
        &mut state,
        2,
        &current,
        2,
        text_view("fresh"),
    );
    assert_eq!(
        state.session.as_ref().unwrap().navigation.stack[0].revision,
        2
    );
    drop(old_guard);
}

#[test]
fn queued_action_survives_selection_revision_without_changing_its_target() {
    let mut navigation = _input_navigation();
    let selection = _input_request(
        1,
        nanika_protocol::ViewEvent::SelectionChanged {
            item_id: Some("two".to_owned()),
        },
    );
    navigation.authorize_input(&selection, None).unwrap();
    navigation.begin().unwrap();
    let route = navigation.stack.last_mut().unwrap();
    route.revision = 2;
    let View::List { list } = std::sync::Arc::make_mut(&mut route.view) else {
        unreachable!()
    };
    list.selected_item_id = Some("two".to_owned());
    navigation.finish(Ok(()));

    // Enter/double-click was captured before selection's RPC or Channel update.
    let action = _input_request(
        1,
        ViewEvent::ActionInvoked {
            invocation: nanika_protocol::ActionInvocation::Default,
            item_id: Some("two".to_owned()),
            action_id: "copy".to_owned(),
        },
    );
    assert_eq!(
        navigation.authorize_input(&action, None).unwrap().revision,
        2
    );
    // The same revision is still stale for an already-open menu.
    assert!(
        navigation
            .authorize_input(&action, Some(1))
            .unwrap_err()
            .contains("Reopen")
    );
    assert_eq!(
        navigation
            .authorize_input(&action, Some(2))
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn queued_actions_revalidate_removed_disabled_and_unknown_targets_after_invalidation() {
    for (item_id, action_id) in [("missing", "copy"), ("one", "missing"), ("two", "copy")] {
        let mut navigation = _input_navigation();
        let route = navigation.stack.last_mut().unwrap();
        route.revision = 3;
        let View::List { list } = std::sync::Arc::make_mut(&mut route.view) else {
            unreachable!()
        };
        list.sections[0].items[1].actions[0].enabled = false;
        let request = _input_request(
            1,
            ViewEvent::ActionInvoked {
                invocation: nanika_protocol::ActionInvocation::Default,
                item_id: Some(item_id.to_owned()),
                action_id: action_id.to_owned(),
            },
        );
        assert!(navigation.authorize_input(&request, None).is_err());
    }
}

#[test]
fn queued_input_rejects_future_revisions_and_replaced_routes() {
    let mut navigation = _input_navigation();
    let mut request = _input_request(2, ViewEvent::Resumed);
    assert!(
        navigation
            .authorize_input(&request, None)
            .unwrap_err()
            .contains("ahead")
    );
    request.revision = 1;
    navigation
        .apply(
            "test.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Pop,
        )
        .unwrap();
    navigation
        .apply(
            "test.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Push {
                view_id: "replacement".to_owned(),
                revision: 1,
                view: Box::new(text_view("new")),
            },
        )
        .unwrap();
    assert!(navigation.authorize_input(&request, None).is_err());
}

fn _input_navigation() -> crate::NavigationState {
    let mut navigation = crate::NavigationState::default();
    navigation
        .apply(
            "test.extension",
            1,
            1,
            nanika_protocol::NavigationEffect::Push {
                view_id: "clipboard".to_owned(),
                revision: 1,
                view: Box::new(View::List {
                    list: Box::new(nanika_protocol::ListView {
                        title: "Clipboard".to_owned(),
                        search_placeholder: "Search".to_owned(),
                        search_text: String::new(),
                        layout: nanika_protocol::ListLayout::Plain,
                        sections: vec![nanika_protocol::ListSection {
                            id: "items".to_owned(),
                            title: None,
                            items: ["one", "two"]
                                .into_iter()
                                .map(|id| nanika_protocol::ListItem {
                                    id: id.to_owned(),
                                    title: id.to_owned(),
                                    subtitle: None,
                                    icon: None,
                                    actions: vec![nanika_protocol::Action::primary("copy", "Copy")],
                                })
                                .collect(),
                        }],
                        selected_item_id: Some("one".to_owned()),
                        detail: None,
                        filter: None,
                        pagination: None,
                    }),
                }),
            },
        )
        .unwrap();
    navigation
}

fn _input_request(revision: u64, event: ViewEvent) -> crate::ViewEventRequest {
    crate::ViewEventRequest {
        session_id: 1,
        route_id: 1,
        revision,
        operation: crate::ViewOperation::Event { event },
    }
}

#[test]
fn action_policy_is_enforced_for_list_and_detail_and_confirmation_expires() {
    use nanika_protocol::{ActionInvocation, ActionStyle};
    let mut navigation = _input_navigation();
    let route = navigation.stack.last_mut().unwrap();
    let View::List { list } = std::sync::Arc::make_mut(&mut route.view) else {
        unreachable!()
    };
    let action = &mut list.sections[0].items[0].actions[0];
    action.allow_default_execution = false;
    let mut detail = text_view("policy");
    let View::Detail { detail: content } = &mut detail else {
        unreachable!()
    };
    content.actions = vec![action.clone()];
    for (view, item_id) in [(&*route.view, Some("one".to_owned())), (&detail, None)] {
        let event = |invocation| ViewEvent::ActionInvoked {
            item_id: item_id.clone(),
            action_id: "copy".to_owned(),
            invocation,
        };
        assert!(authorize_view_event(view, &event(ActionInvocation::Default)).is_err());
        assert!(authorize_view_event(view, &event(ActionInvocation::Explicit)).is_ok());
    }
    let View::List { list } = std::sync::Arc::make_mut(&mut route.view) else {
        unreachable!()
    };
    let action = &mut list.sections[0].items[0].actions[0];
    action.style = ActionStyle::Destructive;
    action.confirmation_title = Some("Copy now?".to_owned());
    let request = |invocation| {
        _input_request(
            1,
            ViewEvent::ActionInvoked {
                item_id: Some("one".to_owned()),
                action_id: "copy".to_owned(),
                invocation,
            },
        )
    };
    assert!(
        navigation
            .authorize_input(&request(ActionInvocation::Explicit), None)
            .is_err()
    );
    assert!(
        navigation
            .authorize_input(&request(ActionInvocation::Confirmed), None)
            .is_ok()
    );
    navigation.stack.last_mut().unwrap().revision = 2;
    assert!(
        navigation
            .authorize_input(&request(ActionInvocation::Confirmed), None)
            .unwrap_err()
            .contains("Confirm the action again")
    );
}

#[test]
fn new_search_clears_action_errors() {
    let mut state = crate::NavigationState::default();
    state.begin().unwrap();
    state.finish(Err("Permission denied".into()));
    let revision = state.snapshot().revision;
    state.clear_error();
    assert!(state.snapshot().error.is_none());
    assert!(state.snapshot().revision > revision);
}

#[test]
fn page_cursors_are_authorized_for_their_exact_surface() {
    use nanika_protocol::{ViewPageTarget, ViewPagination};
    let mut navigation = _input_navigation();
    let view = std::sync::Arc::make_mut(&mut navigation.stack.last_mut().unwrap().view);
    let View::List { list } = view else {
        unreachable!()
    };
    list.pagination = Some(ViewPagination {
        label: "1-10 of 20".into(),
        previous_cursor: None,
        next_cursor: Some("rows-next".into()),
    });
    list.detail = Some(DetailView {
        title: None,
        content: DetailContent::Text {
            value: "page".into(),
            pagination: Some(Box::new(ViewPagination {
                label: "1-4 of 8".into(),
                previous_cursor: None,
                next_cursor: Some("text-next".into()),
            })),
        },
        metadata: vec![],
        actions: vec![],
    });
    for (target, cursor, allowed) in [
        (ViewPageTarget::List, "rows-next", true),
        (ViewPageTarget::Detail, "text-next", true),
        (ViewPageTarget::List, "text-next", false),
        (ViewPageTarget::Detail, "rows-next", false),
        (ViewPageTarget::List, "invented", false),
    ] {
        assert_eq!(
            authorize_view_event(
                view,
                &ViewEvent::PageChanged {
                    target,
                    cursor: cursor.into()
                }
            )
            .is_ok(),
            allowed
        );
    }
}

#[test]
fn page_request_from_an_older_revision_cannot_apply_to_a_new_selection() {
    let mut navigation = _input_navigation();
    let route = navigation.stack.last_mut().unwrap();
    route.revision = 2;
    let View::List { list } = std::sync::Arc::make_mut(&mut route.view) else {
        unreachable!()
    };
    list.selected_item_id = Some("two".into());
    list.detail = Some(DetailView {
        title: None,
        content: DetailContent::Text {
            value: "second item".into(),
            pagination: Some(Box::new(nanika_protocol::ViewPagination {
                label: "1-10 of 20".into(),
                previous_cursor: None,
                next_cursor: Some("10".into()),
            })),
        },
        metadata: vec![],
        actions: vec![],
    });
    let event = ViewEvent::PageChanged {
        target: nanika_protocol::ViewPageTarget::Detail,
        cursor: "10".into(),
    };
    assert!(
        navigation
            .authorize_input(&_input_request(1, event.clone()), None)
            .is_err()
    );
    assert!(
        navigation
            .authorize_input(&_input_request(2, event), None)
            .is_ok()
    );
}

#[test]
fn pasted_control_characters_have_the_same_search_contract_as_view_state() {
    let navigation = _input_navigation();
    let event = ViewEvent::SearchChanged {
        text: "\u{1b}[31m".into(),
    };
    assert!(
        navigation
            .authorize_input(&_input_request(1, event), None)
            .is_ok()
    );
}

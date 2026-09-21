use nanika_protocol::{DetailContent, DetailView, View, ViewEvent};

use crate::authorize_view_event;

#[test]
fn resumed_events_are_authorized_for_every_host_rendered_view() {
    let view = View::Detail {
        detail: DetailView {
            title: None,
            content: DetailContent::Text {
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
                value: value.to_owned(),
            },
            metadata: Vec::new(),
            actions: Vec::new(),
        },
    }
}

#[test]
fn completed_view_is_presented_even_when_execution_recording_failed() {
    let mut navigation = crate::NavigationState::default();
    let completion = nanika_host::RuntimeInvocationCompletion {
        outcome: nanika_host::ExtensionInvocationOutcome::Completed {
            effect: nanika_protocol::NavigationEffect::Push {
                view_id: "created-view".to_owned(),
                revision: 1,
                view: Box::new(text_view("created")),
            },
            has_output: false,
        },
        recording_error: Some("could not record completed action: disk full".to_owned()),
    };
    let error = crate::apply_invocation_completion(completion, |effect| {
        navigation.apply("test.extension", 1, effect)
    })
    .unwrap_err();
    assert_eq!(navigation.stack.last().unwrap().view_id, "created-view");
    assert!(error.contains("disk full"));
}

#[test]
fn recording_failure_does_not_skip_retired_view_cleanup_or_hide_its_error() {
    let mut cleanup_attempted = false;
    let completion = nanika_host::RuntimeInvocationCompletion {
        outcome: nanika_host::ExtensionInvocationOutcome::Completed {
            effect: nanika_protocol::NavigationEffect::Push {
                view_id: "retired-view".to_owned(),
                revision: 1,
                view: Box::new(text_view("retired")),
            },
            has_output: false,
        },
        recording_error: Some("recording failed: disk full".to_owned()),
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
    assert!(error.contains("disk full"));
}

#[test]
fn navigation_rejects_overflow_without_discarding_existing_routes() {
    let mut navigation = crate::NavigationState::default();
    for index in 0..crate::MAX_NAVIGATION_DEPTH {
        navigation
            .apply(
                "test.extension",
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
        .apply("test.extension", 1, nanika_protocol::NavigationEffect::Pop)
        .unwrap();
    navigation
        .apply(
            "test.extension",
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

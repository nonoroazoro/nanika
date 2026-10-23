use nanika_protocol::{Action, ActionInvocation, ActionStyle, validate_actions};

#[test]
fn execution_policy_is_independent_of_visual_style() {
    for style in [
        ActionStyle::Primary,
        ActionStyle::Secondary,
        ActionStyle::Destructive
    ] {
        let mut action = Action::primary("run", "Run");
        action.style = style;
        for allowed in [false, true] {
            action.allow_default_execution = allowed;
            validate_actions(&[action.clone()]).unwrap();
            assert_eq!(action.allows_invocation(ActionInvocation::Default), allowed);
            assert!(action.allows_invocation(ActionInvocation::Explicit));
        }
        action.enabled = false;
        for invocation in [
            ActionInvocation::Default,
            ActionInvocation::Explicit,
            ActionInvocation::Confirmed
        ] {
            assert!(!action.allows_invocation(invocation));
        }
    }
}

#[test]
fn confirmation_requires_acknowledgement_and_disallows_default_execution() {
    let mut action = Action::primary("clear", "Clear");
    action.style = ActionStyle::Destructive;
    action.confirmation_title = Some("Clear now?".into());
    assert!(validate_actions(&[action.clone()]).is_err());
    assert!(!action.allows_invocation(ActionInvocation::Default));
    action.allow_default_execution = false;
    validate_actions(&[action.clone()]).unwrap();
    assert!(!action.allows_invocation(ActionInvocation::Default));
    assert!(!action.allows_invocation(ActionInvocation::Explicit));
    assert!(action.allows_invocation(ActionInvocation::Confirmed));
}

#[test]
fn wire_policy_and_invocation_intent_are_required() {
    let action = Action::primary("open", "Open");
    let mut value = serde_json::to_value(&action).unwrap();
    assert_eq!(
        serde_json::from_value::<Action>(value.clone()).unwrap(),
        action
    );
    value
        .as_object_mut()
        .unwrap()
        .remove("allow_default_execution");
    assert!(serde_json::from_value::<Action>(value).is_err());
    let mut event = serde_json::json!({
        "kind": "actionInvoked", "item_id": null, "action_id": "open"
    });
    assert!(serde_json::from_value::<nanika_protocol::ViewEvent>(event.clone()).is_err());
    event["invocation"] = serde_json::json!("unknown");
    assert!(serde_json::from_value::<nanika_protocol::ViewEvent>(event).is_err());
}

#[test]
fn semantic_icons_are_closed_data_not_paths_or_markup() {
    let mut action = Action::primary("open", "Open");
    action.icon = Some(nanika_protocol::ActionIcon::Power);
    let mut value = serde_json::to_value(&action).unwrap();
    assert_eq!(value["icon"], "power");
    assert_eq!(
        serde_json::from_value::<Action>(value.clone()).unwrap(),
        action
    );
    for invalid in [
        "https://example.com/icon.svg",
        "../icon.svg",
        "<svg/>",
        "unknown"
    ] {
        value["icon"] = serde_json::json!(invalid);
        assert!(serde_json::from_value::<Action>(value.clone()).is_err());
    }
}

#[test]
fn localized_actions_keep_policy_and_reject_conflicting_group_definitions() {
    use nanika_protocol::{PresentationGroup, message};
    let mut first = Action::primary("first", message("run", []));
    first.group = Some(PresentationGroup {
        id: "operations".into(),
        title: message("group", [])
    });
    let mut second = first.clone();
    second.id = "second".into();
    validate_actions(&[first.clone(), second.clone()]).unwrap();
    second.group.as_mut().unwrap().title = "Different title".into();
    assert!(
        validate_actions(&[first.clone(), second.clone()])
            .unwrap_err()
            .contains("conflicting")
    );
    second.group.as_mut().unwrap().id = "another".into();
    validate_actions(&[first.clone(), second]).unwrap();
    let mut encoded = serde_json::to_value(&first).unwrap();
    assert_eq!(
        serde_json::from_value::<Action>(encoded.clone()).unwrap(),
        first
    );
    encoded["group"] = serde_json::json!("legacy label");
    assert!(serde_json::from_value::<Action>(encoded).is_err());
    first.confirmation_title = Some(message("confirm", []));
    assert!(validate_actions(&[first.clone()]).is_err());
    first.allow_default_execution = false;
    validate_actions(&[first.clone()]).unwrap();
    assert!(!first.allows_invocation(ActionInvocation::Default));
    assert!(!first.allows_invocation(ActionInvocation::Explicit));
    assert!(first.allows_invocation(ActionInvocation::Confirmed));
}

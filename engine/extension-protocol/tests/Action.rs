use nanika_protocol::{Action, ActionInvocation, ActionStyle, validate_actions};

#[test]
fn execution_policy_is_independent_of_visual_style() {
    for style in [
        ActionStyle::Primary,
        ActionStyle::Secondary,
        ActionStyle::Destructive,
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
            ActionInvocation::Confirmed,
        ] {
            assert!(!action.allows_invocation(invocation));
        }
    }
}

#[test]
fn confirmation_requires_acknowledgement_and_disallows_default_execution() {
    let mut action = Action::primary("clear", "Clear");
    action.style = ActionStyle::Destructive;
    action.confirmation_title = Some("Clear now?".to_owned());
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

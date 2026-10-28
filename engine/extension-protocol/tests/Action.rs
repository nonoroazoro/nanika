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

#[test]
fn shortcuts_are_optional_structured_and_validate_key_identity() {
    use nanika_protocol::{ActionShortcut, ShortcutModifier};
    let mut action = Action::primary("custom.reveal", "Reveal");
    assert!(
        serde_json::to_value(&action)
            .unwrap()
            .get("shortcut")
            .is_none()
    );
    action.shortcut = Some(Box::new(ActionShortcut::primary("Enter")));
    action.allow_default_execution = false;
    validate_actions(&[action.clone()]).unwrap();
    let encoded = serde_json::to_value(&action).unwrap();
    assert_eq!(
        encoded["shortcut"],
        serde_json::json!({"key":{"kind":"virtual","value":"Enter"},"modifiers":["primary"]})
    );
    assert_eq!(serde_json::from_value::<Action>(encoded).unwrap(), action);
    assert!(!action.allows_invocation(ActionInvocation::Default));
    assert!(action.allows_invocation(ActionInvocation::Explicit));
    for key in [
        "Escape",
        "Tab",
        "ArrowDown",
        "Comma",
        "KeyAA",
        "Digit10",
        "enter"
    ] {
        action.shortcut = Some(Box::new(ActionShortcut::primary(key)));
        assert!(validate_actions(&[action.clone()]).is_err(), "{key}");
    }
    for modifiers in [
        vec![],
        vec![ShortcutModifier::Shift],
        vec![ShortcutModifier::Primary; 2]
    ] {
        action.shortcut = Some(Box::new(ActionShortcut {
            key: nanika_protocol::ShortcutKey::Virtual("K".into()),
            modifiers
        }));
        assert!(validate_actions(&[action.clone()]).is_err());
    }
    assert!(
        serde_json::from_value::<ActionShortcut>(
            serde_json::json!({"key":{"kind":"virtual","value":"K"}, "modifiers":["unknown"]})
        )
        .is_err()
    );
}

#[test]
fn shortcuts_are_unique_per_item_regardless_of_modifier_order() {
    use nanika_protocol::{
        ActionShortcut,
        ShortcutModifier::{Primary, Shift}
    };
    let mut first = Action::primary("first", "First");
    first.shortcut = Some(Box::new(ActionShortcut {
        key: nanika_protocol::ShortcutKey::Virtual("K".into()),
        modifiers: vec![Primary, Shift]
    }));
    let mut second = Action::primary("second", "Second");
    second.shortcut = Some(Box::new(ActionShortcut {
        key: nanika_protocol::ShortcutKey::Virtual("K".into()),
        modifiers: vec![Shift, Primary]
    }));
    assert!(
        validate_actions(&[first.clone(), second.clone()])
            .unwrap_err()
            .contains("shortcuts must be unique")
    );
    second.shortcut.as_mut().unwrap().key = nanika_protocol::ShortcutKey::Virtual("1".into());
    validate_actions(&[first.clone(), second.clone()]).unwrap();
    validate_actions(&[first]).unwrap();
    validate_actions(&[second]).unwrap();
}

#[test]
fn extensions_can_bind_editing_keys_and_distinguish_physical_keys() {
    use nanika_protocol::{ActionShortcut, ShortcutKey};
    for key in ["A", "C", "V", "X", "Z", "Q", "W"] {
        let mut action = Action::primary("custom", "Custom");
        action.shortcut = Some(Box::new(ActionShortcut::primary(key)));
        validate_actions(&[action]).unwrap();
    }
    let mut logical = Action::primary("logical", "Logical");
    logical.shortcut = Some(Box::new(ActionShortcut::primary("I")));
    let mut physical = Action::primary("physical", "Physical");
    physical.shortcut = Some(Box::new(ActionShortcut {
        key: ShortcutKey::Physical("KeyI".into()),
        modifiers: vec![nanika_protocol::ShortcutModifier::Primary]
    }));
    validate_actions(&[logical, physical.clone()]).unwrap();
    assert_eq!(
        serde_json::from_value::<Action>(serde_json::to_value(&physical).unwrap()).unwrap(),
        physical
    );
    physical.shortcut.as_mut().unwrap().key = ShortcutKey::Physical("I".into());
    assert!(validate_actions(&[physical]).is_err());
}

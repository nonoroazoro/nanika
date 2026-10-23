use nanika_extension_package::ConfigurationContribution;
use nanika_platform::TargetPlatform;
use serde_json::json;

#[test]
fn platform_visibility_filters_presentation_without_changing_defaults() {
    let contribution: ConfigurationContribution = serde_json::from_value(json!({
        "title": "Platform settings",
        "properties": {
            "common": {"type": "boolean", "title": "Common", "persistence": "beforeApply", "default": true},
            "windows": {"type": "boolean", "title": "Windows", "persistence": "beforeApply", "default": false, "platforms": ["windows"], "order": 20},
            "macos": {"type": "boolean", "title": "macOS", "persistence": "beforeApply", "default": false, "platforms": ["macos"], "order": 10}
        }
    })).unwrap();
    contribution.validate().unwrap();
    for platform in [TargetPlatform::Windows, TargetPlatform::Macos] {
        assert_eq!(serde_json::to_value(platform).unwrap(), platform.as_str());
        let visible = contribution.for_platform(platform);
        assert_eq!(visible.properties.len(), 2);
        assert!(visible.properties.contains_key("common"));
        assert!(visible.properties.contains_key(platform.as_str()));
        visible.validate().unwrap();
    }
    assert_eq!(contribution.defaults().len(), 3);
}

#[test]
fn rejects_duplicate_and_excessive_platforms() {
    for platforms in [
        json!(["windows", "windows"]),
        json!(["macos", "macos"]),
        json!(["windows", "macos", "windows"])
    ] {
        let contribution: ConfigurationContribution = serde_json::from_value(json!({
            "title": "Platform settings",
            "properties": {"enabled": {"type": "boolean", "title": "Enabled", "persistence": "beforeApply", "default": true, "platforms": platforms}}
        })).unwrap();
        assert!(contribution.validate().is_err());
    }
}

#[test]
fn unknown_platforms_are_rejected_during_deserialization() {
    let contribution = serde_json::from_value::<ConfigurationContribution>(json!({
        "title": "Platform settings",
        "properties": {"enabled": {"type": "boolean", "title": "Enabled", "persistence": "beforeApply", "default": true, "platforms": ["linux"]}}
    }));
    assert!(contribution.is_err());
}

#[test]
fn settings_actions_are_explicit_operations_outside_configuration_values() {
    let mut contribution: ConfigurationContribution = serde_json::from_value(json!({
        "title": "Operations", "properties": {},
        "actions": [{
            "description": "Delete stored data.",
            "action": { "id": "reset", "title": "Reset", "style": "destructive",
                "enabled": true, "allow_default_execution": false,
                "confirmation_title": "Reset all data?", "group": {"id":"danger-zone","title":"Danger zone"} }
        }]
    }))
    .unwrap();
    contribution.validate().unwrap();
    assert!(contribution.defaults().is_empty());
    contribution.validate_values(&Default::default()).unwrap();
    let valid = contribution.actions[0].clone();
    contribution.actions[0].action.confirmation_title = None;
    assert!(contribution.validate().is_err());
    contribution.actions[0] = valid.clone();
    contribution.actions[0].action.allow_default_execution = true;
    assert!(contribution.validate().is_err());
    contribution.actions[0] = valid.clone();
    contribution.actions.push(valid);
    assert!(contribution.validate().is_err());
}

#[test]
fn quick_menu_requires_bounded_unique_references_to_boolean_settings_or_actions() {
    let base = json!({
        "title": "Fixture", "properties": {
            "enabled": {"type": "boolean", "title": "Enabled", "persistence": "afterApply", "default": true},
            "limit": {"type": "integer", "title": "Limit", "persistence": "beforeApply", "default": 1, "minimum": 1, "maximum": 10, "multipleOf": 1}
        },
        "actions": [{"description": "Delete data.", "action": {"id": "clear", "title": "Clear", "style": "destructive", "enabled": true, "allow_default_execution": false, "confirmation_title": "Clear all?"}}],
        "quickMenu": [
            {"id": "appearance", "title": "Appearance", "items": [{"kind": "toggle", "key": "enabled"}]},
            {"id": "danger", "title": "Danger zone", "items": [{"kind": "action", "id": "clear"}]}
        ]
    });
    serde_json::from_value::<ConfigurationContribution>(base.clone())
        .unwrap()
        .validate()
        .unwrap();
    for items in [
        json!([]),
        json!([{"kind": "toggle", "key": "missing"}]),
        json!([{"kind": "toggle", "key": "limit"}]),
        json!([{"kind": "action", "id": "missing"}]),
        json!([{"kind": "toggle", "key": "enabled"}, {"kind": "toggle", "key": "enabled"}])
    ] {
        let mut candidate = base.clone();
        candidate["quickMenu"][0]["items"] = items;
        assert!(
            serde_json::from_value::<ConfigurationContribution>(candidate)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    let mut oversized = base.clone();
    oversized["quickMenu"] = json!(vec![base["quickMenu"][0].clone(); 9]);
    assert!(
        serde_json::from_value::<ConfigurationContribution>(oversized)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut platform = base;
    platform["properties"]["enabled"]["platforms"] = json!(["windows"]);
    let contribution: ConfigurationContribution = serde_json::from_value(platform).unwrap();
    let visible = contribution.for_platform(TargetPlatform::Macos);
    visible.validate().unwrap();
    assert_eq!(visible.quick_menu.len(), 1);
    assert_eq!(visible.quick_menu[0].title, "Danger zone".into());
}

#[test]
fn group_identity_is_explicit_and_conflicting_definitions_are_rejected() {
    let base = json!({"title":"Settings", "properties": {
        "first": {"title":"First", "type":"boolean", "default":true, "persistence":"beforeApply", "group":{"id":"first", "title":"Options"}},
        "second": {"title":"Second", "type":"boolean", "default":false, "persistence":"beforeApply", "group":{"id":"second", "title":"Options"}}
    }, "quickMenu":[{"id":"first", "title":"Options", "items":[{"kind":"toggle", "key":"first"}]}]});
    serde_json::from_value::<ConfigurationContribution>(base.clone())
        .unwrap()
        .validate()
        .unwrap();
    let mut conflict = base.clone();
    conflict["properties"]["second"]["group"] = json!({"id":"first", "title":"Different"});
    assert!(
        serde_json::from_value::<ConfigurationContribution>(conflict)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut conflict = base.clone();
    conflict["quickMenu"][0]["title"] = json!("Different");
    assert!(
        serde_json::from_value::<ConfigurationContribution>(conflict)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut invalid = base.clone();
    invalid["properties"]["first"]["group"]["id"] = json!("$configuration");
    assert!(
        serde_json::from_value::<ConfigurationContribution>(invalid)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut legacy = base;
    legacy["properties"]["first"]["group"] = json!("Options");
    assert!(serde_json::from_value::<ConfigurationContribution>(legacy).is_err());
}

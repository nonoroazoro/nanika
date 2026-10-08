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
        json!(["windows", "macos", "windows"]),
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

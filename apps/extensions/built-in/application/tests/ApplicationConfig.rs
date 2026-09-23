use std::collections::BTreeMap;

use nanika_protocol::ExtensionConfiguration;

use crate::ApplicationConfig;

#[test]
fn parses_host_configuration() {
    let root = std::env::temp_dir().join("nanika-applications");
    let configuration = ExtensionConfiguration::new(BTreeMap::from([(
        "application.roots".to_owned(),
        serde_json::json!([root.clone()]),
    )]));

    let config = ApplicationConfig::from_configuration(&configuration)
        .expect("host configuration should be valid");

    assert_eq!(config.roots, vec![root.clone()]);
    assert!(config.exclusions.is_empty());
}

#[test]
fn rejects_relative_paths() {
    let configuration = ExtensionConfiguration::new(BTreeMap::from([(
        "application.roots".to_owned(),
        serde_json::json!(["relative"]),
    )]));

    assert!(ApplicationConfig::from_configuration(&configuration).is_err());
}

#[test]
fn manifest_defaults_enable_builtin_sources_and_explicit_false_disables_one() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../manifest.jsonc")).unwrap();
    let mut values: BTreeMap<String, serde_json::Value> = manifest["contributes"]["configuration"]
        ["properties"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, property)| (key.clone(), property["default"].clone()))
        .collect();
    let config =
        ApplicationConfig::from_configuration(&ExtensionConfiguration::new(values.clone()))
            .unwrap();
    let expected: std::collections::BTreeSet<String> = values
        .iter()
        .filter(|(_, value)| **value == true)
        .map(|(key, _)| key.clone())
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(config.enabled_builtin_roots, expected);
    let key = expected.first().unwrap();
    values.insert(key.clone(), serde_json::json!(false));
    let disabled =
        ApplicationConfig::from_configuration(&ExtensionConfiguration::new(values.clone()))
            .unwrap();
    assert!(!disabled.enabled_builtin_roots.contains(key));
    assert_eq!(disabled.enabled_builtin_roots.len(), expected.len() - 1);
    values.insert(key.clone(), serde_json::json!("false"));
    assert!(ApplicationConfig::from_configuration(&ExtensionConfiguration::new(values)).is_err());
}

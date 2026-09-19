use nanika_extension_script::ScriptConfig;
use nanika_protocol::ExtensionConfiguration;
use std::collections::BTreeMap;

#[test]
fn parses_directory_configuration() {
    let root = std::env::temp_dir().join("nanika-scripts");
    let configuration = ExtensionConfiguration::new(BTreeMap::from([(
        "script.roots".to_owned(),
        serde_json::json!([root]),
    )]));
    let config = ScriptConfig::from_configuration(&configuration).unwrap();
    assert_eq!(config.roots, vec![root]);
}

#[test]
fn rejects_missing_relative_and_non_string_directories() {
    assert!(ScriptConfig::from_configuration(&ExtensionConfiguration::default()).is_err());
    for value in [
        serde_json::json!(["relative"]),
        serde_json::json!([1]),
        serde_json::json!("path"),
    ] {
        let configuration =
            ExtensionConfiguration::new(BTreeMap::from([("script.roots".to_owned(), value)]));
        assert!(ScriptConfig::from_configuration(&configuration).is_err());
    }
}

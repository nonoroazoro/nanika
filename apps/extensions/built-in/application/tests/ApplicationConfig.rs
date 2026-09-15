use std::collections::BTreeMap;

use nanika_protocol::ExtensionConfiguration;

use crate::ApplicationConfig;

#[test]
fn parses_host_configuration() {
    let root = std::env::temp_dir().join("nanika-applications");
    let configuration = ExtensionConfiguration::new(BTreeMap::from([
        (
            "application.exclusions".to_owned(),
            serde_json::json!([root.join("Excluded")]),
        ),
        (
            "application.roots".to_owned(),
            serde_json::json!([root.clone()]),
        ),
    ]));

    let config = ApplicationConfig::from_configuration(&configuration)
        .expect("host configuration should be valid");

    assert_eq!(config.roots, vec![root.clone()]);
    assert_eq!(config.exclusions, vec![root.join("Excluded")]);
}

#[test]
fn rejects_relative_paths() {
    let configuration = ExtensionConfiguration::new(BTreeMap::from([
        ("application.exclusions".to_owned(), serde_json::json!([])),
        (
            "application.roots".to_owned(),
            serde_json::json!(["relative"]),
        ),
    ]));

    assert!(ApplicationConfig::from_configuration(&configuration).is_err());
}

use std::collections::BTreeMap;

use nanika_config::{ConfigStore, ExtensionConfigurationFile};
use nanika_extension_package::ConfigurationContribution;

use crate::ExtensionConfigurationRegistry;

#[test]
fn host_loads_defaults_validates_updates_and_persists_values() {
    let root = std::env::temp_dir().join(format!(
        "nanika-extension-configuration-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(root.join("machine"), root.join("configuration"))
        .expect("configuration store");
    let registry = ExtensionConfigurationRegistry::new(store.clone());
    let contribution = contribution();

    let initial = registry
        .register("com.example.fixture", Some(&contribution))
        .expect("configuration should register");
    assert_eq!(initial.values()["fixture.count"], 50);

    let invalid = registry.update(
        "com.example.fixture",
        BTreeMap::from([("fixture.count".to_owned(), serde_json::json!(0))]),
    );
    assert!(invalid.is_err());

    let updated = registry
        .update(
            "com.example.fixture",
            BTreeMap::from([("fixture.count".to_owned(), serde_json::json!(100))]),
        )
        .expect("valid configuration should persist");
    assert_eq!(updated.values()["fixture.count"], 100);
    let stored = store
        .load::<ExtensionConfigurationFile>(
            store.extension_configuration_file("com.example.fixture"),
        )
        .expect("persisted configuration");
    assert_eq!(stored.values["fixture.count"], 100);

    std::fs::remove_dir_all(root).expect("test root should be removable");
}

fn contribution() -> ConfigurationContribution {
    serde_json::from_value(serde_json::json!({
        "title": "Fixture",
        "properties": {
            "fixture.count": {
                "type": "integer",
                "title": "Count",
                "default": 50,
                "minimum": 1,
                "maximum": 5000,
                "multipleOf": 1
            }
        }
    }))
    .expect("configuration schema")
}

#[test]
fn platform_scoped_saves_preserve_hidden_values_and_reject_incomplete_forms() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/test-work")
        .join(format!("configuration-platform-{}", std::process::id()));
    let store = ConfigStore::open(root.join("machine"), root.join("configuration")).unwrap();
    let registry = ExtensionConfigurationRegistry::new(store.clone());
    let contribution: ConfigurationContribution = serde_json::from_value(serde_json::json!({
        "title": "Platform settings",
        "properties": {
            "fixture.count": {"type": "integer", "title": "Count", "default": 1, "minimum": 1, "maximum": 100, "multipleOf": 1},
            "fixture.windows": {"type": "boolean", "title": "Windows", "default": true, "platforms": ["windows"]},
            "fixture.macos": {"type": "boolean", "title": "macOS", "default": true, "platforms": ["macos"]}
        }
    })).unwrap();
    let mut stored_values = contribution.defaults();
    stored_values.insert("fixture.windows".to_owned(), serde_json::json!(false));
    stored_values.insert("fixture.macos".to_owned(), serde_json::json!(false));
    let path = store.extension_configuration_file("com.example.platform");
    store
        .save(
            &path,
            &ExtensionConfigurationFile::new(stored_values.clone()),
        )
        .unwrap();
    registry
        .register("com.example.platform", Some(&contribution))
        .unwrap();
    let snapshot = registry.snapshots().remove(0);
    assert_eq!(snapshot.values.len(), 2);
    assert_eq!(snapshot.contribution.properties.len(), 2);
    let hidden_key = stored_values
        .keys()
        .find(|key| !snapshot.values.contains_key(*key))
        .unwrap();

    let mut submitted = snapshot.values;
    submitted.insert("fixture.count".to_owned(), serde_json::json!(5));
    let effective = registry
        .update("com.example.platform", submitted.clone())
        .unwrap();
    assert_eq!(effective.values()[hidden_key], false);
    assert_eq!(effective.values()["fixture.count"], 5);
    assert_eq!(registry.snapshots()[0].values, submitted);
    let persisted = std::fs::read(&path).unwrap();
    let reopened = ExtensionConfigurationRegistry::new(store.clone());
    assert_eq!(
        reopened
            .register("com.example.platform", Some(&contribution))
            .unwrap(),
        effective
    );

    let mut unknown = submitted.clone();
    unknown.insert("fixture.unknown".to_owned(), serde_json::json!(true));
    let mut invalid_hidden = submitted.clone();
    invalid_hidden.insert(hidden_key.clone(), serde_json::json!("false"));
    for invalid in [
        BTreeMap::new(),
        BTreeMap::from([("fixture.count".to_owned(), serde_json::json!(8))]),
        unknown,
        invalid_hidden,
    ] {
        assert!(registry.update("com.example.platform", invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), persisted);
        assert_eq!(registry.snapshots()[0].values, submitted);
    }
    std::fs::remove_dir_all(root).unwrap();
}

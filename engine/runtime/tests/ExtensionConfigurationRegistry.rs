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

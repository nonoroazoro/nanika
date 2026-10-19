use nanika_config::{ConfigStore, ExtensionRegistryConfig, ExtensionRegistryTransaction};

#[test]
fn registry_round_trips_enablement_in_the_config_tree() {
    let root = temporary_root("round-trip");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).expect("store");
    let mut registry = ExtensionRegistryTransaction::begin(&store).expect("registry transaction");
    registry.set_enabled("com.example.extension", false);
    registry.save().expect("save registry");

    drop(registry);
    let loaded = ExtensionRegistryConfig::load(&store).expect("load registry");
    assert!(!loaded.is_enabled("com.example.extension"));
    assert!(loaded.is_enabled("com.nanika.script"));

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn registry_updates_preserve_nested_comments_and_formatting() {
    let root = temporary_root("comments");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).expect("store");
    std::fs::write(
        store.extensions_file(),
        r#"{
  "formatVersion": 1,
  "extensions": {
    // Keep this explanation.
    "com.example.extension": true,
    "com.example.unchanged": false,
  },
}
"#
    )
    .expect("registry fixture");
    let mut registry = ExtensionRegistryTransaction::begin(&store).expect("registry transaction");
    registry.set_enabled("com.example.extension", false);
    registry.save().expect("save registry");

    drop(registry);
    let saved = std::fs::read_to_string(store.extensions_file()).expect("saved registry");
    assert!(saved.contains("// Keep this explanation."));
    assert!(saved.contains("\"com.example.unchanged\": false"));
    assert!(saved.contains("\"com.example.extension\": false"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn unreadable_registry_uses_defaults_without_replacing_the_path() {
    let root = temporary_root("strict-missing");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).expect("store");
    std::fs::create_dir_all(store.extensions_file()).expect("registry directory fixture");

    assert_eq!(
        ExtensionRegistryConfig::load(&store).unwrap(),
        ExtensionRegistryConfig::default()
    );
    assert!(store.extensions_file().is_dir());
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn temporary_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nanika-extension-registry-{name}-{}",
        std::process::id()
    ))
}

#[test]
fn invalid_enablement_does_not_discard_valid_options_or_block_settings_saves() {
    let root = temporary_root("invalid-options");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).unwrap();
    for text in [
        r#"{"formatVersion":1,"extensions":{"com.example.valid":false,"com.example.invalid":"wrong"}}"#,
        "{ malformed"
    ] {
        std::fs::write(store.extensions_file(), text).unwrap();
        let mut transaction = ExtensionRegistryTransaction::begin(&store).unwrap();
        assert_eq!(
            transaction.is_enabled("com.example.valid"),
            text == "{ malformed"
        );
        assert!(transaction.is_enabled("com.example.invalid"));
        assert_eq!(
            std::fs::read_to_string(store.extensions_file()).unwrap(),
            text
        );
        transaction.set_enabled("com.example.invalid", false);
        transaction.save().unwrap();
        assert!(
            !ExtensionRegistryConfig::load(&store)
                .unwrap()
                .is_enabled("com.example.invalid")
        );
        drop(transaction);
    }
    std::fs::remove_dir_all(root).unwrap();
}

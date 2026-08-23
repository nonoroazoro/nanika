use nanika_config::{ConfigStore, ExtensionRegistryConfig};

#[test]
fn registry_round_trips_enablement_in_the_config_tree() {
    let root = temporary_root("round-trip");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).expect("store");
    let mut registry = ExtensionRegistryConfig::default();
    registry.set_enabled("com.example.extension", false);
    registry.save(&store).expect("save registry");

    let loaded = ExtensionRegistryConfig::load(&store).expect("load registry");
    assert!(!loaded.is_enabled("com.example.extension", true));
    assert!(loaded.is_enabled("com.nanika.command", true));

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
"#,
    )
    .expect("registry fixture");
    let mut registry = ExtensionRegistryConfig::load(&store).expect("load registry");
    registry.set_enabled("com.example.extension", false);
    registry.save(&store).expect("save registry");

    let saved = std::fs::read_to_string(store.extensions_file()).expect("saved registry");
    assert!(saved.contains("// Keep this explanation."));
    assert!(saved.contains("\"com.example.unchanged\": false"));
    assert!(saved.contains("\"com.example.extension\": false"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn registry_defaults_only_when_the_file_is_missing() {
    let root = temporary_root("strict-missing");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).expect("store");
    std::fs::create_dir_all(store.extensions_file()).expect("registry directory fixture");

    assert!(ExtensionRegistryConfig::load(&store).is_err());
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn temporary_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nanika-extension-registry-{name}-{}",
        std::process::id()
    ))
}

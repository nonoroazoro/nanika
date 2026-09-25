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
    assert!(loaded.is_enabled("com.nanika.command"));

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

#[test]
fn concurrent_transactions_preserve_other_extensions_and_rollback_is_isolated() {
    let root = temporary_root("concurrent");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).unwrap();
    let first = ExtensionRegistryTransaction::begin(&store).unwrap();
    let (started, ready) = std::sync::mpsc::sync_channel(1);
    let (done, complete) = std::sync::mpsc::sync_channel(1);
    let other = store.clone();
    let thread = std::thread::spawn(move || {
        started.send(()).unwrap();
        let mut next = ExtensionRegistryTransaction::begin(&other).unwrap();
        next.set_enabled("com.example.second", false);
        next.save().unwrap();
        done.send(()).unwrap();
    });
    ready.recv().unwrap();
    assert!(
        complete
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err()
    );
    let mut first = first;
    first.set_enabled("com.example.first", false);
    first.save().unwrap();
    drop(first);
    complete
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    thread.join().unwrap();
    let loaded = ExtensionRegistryConfig::load(&store).unwrap();
    assert!(!loaded.is_enabled("com.example.first"));
    assert!(!loaded.is_enabled("com.example.second"));
    let mut transaction = ExtensionRegistryTransaction::begin(&store).unwrap();
    transaction.remove("com.example.first");
    transaction.save().unwrap();
    transaction.rollback().unwrap();
    drop(transaction);
    assert_eq!(ExtensionRegistryConfig::load(&store).unwrap(), loaded);
    std::fs::remove_dir_all(root).unwrap();
}

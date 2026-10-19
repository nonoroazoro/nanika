use nanika_config::{ConfigStore, ExtensionRegistryConfig, ExtensionRegistryTransaction};

#[test]
fn concurrent_transactions_preserve_other_extensions_and_rollback_is_isolated() {
    let root = _temporary_root("concurrent");
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

#[test]
fn rollback_restores_original_bytes_including_invalid_configuration() {
    let root = _temporary_root("original-bytes");
    let store = ConfigStore::open(root.join("machine"), root.join("config")).unwrap();
    for original in [
        br#"{ "formatVersion": 1, "extensions": { "com.example.extension": false } }"#.as_slice(),
        b"{ malformed",
        &[0xff, 0xfe]
    ] {
        std::fs::write(store.extensions_file(), original).unwrap();
        let mut transaction = ExtensionRegistryTransaction::begin(&store).unwrap();
        transaction.set_enabled("com.example.extension", true);
        transaction.save().unwrap();
        transaction.rollback().unwrap();
        assert_eq!(std::fs::read(store.extensions_file()).unwrap(), original);
    }
    std::fs::remove_file(store.extensions_file()).unwrap();
    let mut transaction = ExtensionRegistryTransaction::begin(&store).unwrap();
    transaction.set_enabled("com.example.extension", false);
    transaction.save().unwrap();
    transaction.rollback().unwrap();
    assert!(!store.extensions_file().exists());
    drop(transaction);
    std::fs::remove_dir_all(root).unwrap();
}

fn _temporary_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nanika-registry-transaction-{name}-{}",
        std::process::id()
    ))
}

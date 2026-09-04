use nanika_config::{BootstrapConfig, CONFIG_FORMAT_VERSION, ConfigError, ConfigStore};
use uuid::Uuid;

#[test]
fn bootstrap_is_created_and_reused() {
    let root = std::env::temp_dir().join(format!("nanika-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let first = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let bootstrap: BootstrapConfig = first.load(first.bootstrap_path()).expect("bootstrap");
    assert_eq!(bootstrap.format_version, CONFIG_FORMAT_VERSION);
    let second = ConfigStore::open(&root, root.join("config")).expect("store should reopen");
    let same: BootstrapConfig = second.load(second.bootstrap_path()).expect("bootstrap");
    assert_eq!(same.machine_id, bootstrap.machine_id);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn comments_are_accepted_at_the_typed_boundary() {
    let root = std::env::temp_dir().join(format!("nanika-config-jsonc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let file = store.config_file();
    std::fs::write(
        &file,
        r#"{
          // keep this comment
          "formatVersion": 1,
          "configRoot": "config",
          "machineId": "00000000-0000-0000-0000-000000000000"
        }"#,
    )
    .expect("write JSONC");
    let value: BootstrapConfig = store.load(&file).expect("JSONC should parse");
    assert_eq!(value.machine_id, Uuid::nil());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn malformed_bootstrap_recovers_initial_backup_and_becomes_read_only() {
    let root = std::env::temp_dir().join(format!("nanika-config-recovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let bootstrap: BootstrapConfig = store.load(store.bootstrap_path()).expect("bootstrap");
    std::fs::write(store.bootstrap_path(), "{ malformed").expect("corrupt bootstrap");
    let recovered = ConfigStore::open(&root, root.join("config")).expect("backup should recover");
    assert!(recovered.is_read_only());
    assert!(recovered.save(recovered.config_file(), &bootstrap).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn valid_relocation_refreshes_the_recovery_boundary() {
    let root =
        std::env::temp_dir().join(format!("nanika-config-relocation-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let mut bootstrap: BootstrapConfig = store.load(store.bootstrap_path()).expect("bootstrap");
    bootstrap.config_root = root.join("relocated-config");
    std::fs::write(
        store.bootstrap_path(),
        serde_json::to_string_pretty(&bootstrap).expect("bootstrap should serialize"),
    )
    .expect("relocated bootstrap should save");

    let relocated = ConfigStore::open(&root, root.join("config")).expect("relocation should load");
    assert_eq!(relocated.config_root(), bootstrap.config_root);
    std::fs::write(relocated.bootstrap_path(), "{ malformed")
        .expect("bootstrap should become malformed");
    let recovered = ConfigStore::open(&root, root.join("config")).expect("backup should recover");
    assert!(recovered.is_read_only());
    assert_eq!(recovered.config_root(), bootstrap.config_root);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn invalid_bootstrap_fields_recover_the_last_known_good_file() {
    let root = std::env::temp_dir().join(format!(
        "nanika-config-invalid-bootstrap-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    std::fs::write(
        store.bootstrap_path(),
        r#"{"formatVersion":1,"configRoot":"relative","machineId":"00000000-0000-0000-0000-000000000000"}"#,
    )
    .expect("invalid bootstrap should save");

    let recovered = ConfigStore::open(&root, root.join("config")).expect("backup should recover");
    assert!(recovered.is_read_only());
    assert_eq!(recovered.config_root(), root.join("config"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn backups_preserve_relative_config_paths() {
    let root = std::env::temp_dir().join(format!("nanika-config-backups-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let value: BootstrapConfig = store.load(store.bootstrap_path()).expect("bootstrap");
    for extension in ["one", "two"] {
        let path = store
            .config_root()
            .join("extensions")
            .join(extension)
            .join("settings.jsonc");
        std::fs::create_dir_all(path.parent().expect("settings parent"))
            .expect("settings parent should exist");
        std::fs::write(&path, "{}\n").expect("initial settings should exist");
        store.save(&path, &value).expect("settings should save");
        assert!(
            root.join("backups/config/extensions")
                .join(extension)
                .join("settings.jsonc")
                .is_file()
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn generic_save_cannot_stale_the_bootstrap_boundary() {
    let root = std::env::temp_dir().join(format!("nanika-config-scope-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let bootstrap: BootstrapConfig = store.load(store.bootstrap_path()).expect("bootstrap");
    assert!(store.save(store.bootstrap_path(), &bootstrap).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn relative_machine_roots_are_rejected_without_writing() {
    assert!(ConfigStore::open("relative-machine", "relative-config").is_err());
}

#[test]
fn loads_are_confined_to_configuration_paths() {
    let root = std::env::temp_dir().join(format!(
        "nanika-config-load-boundary-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let outside = root.join("outside.jsonc");
    std::fs::write(&outside, "{}\n").expect("outside file should exist");

    let result = store.load::<serde_json::Value>(&outside);

    assert!(matches!(result, Err(ConfigError::Invalid(_))));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn targeted_updates_preserve_comments_and_validate_the_result() {
    let root = std::env::temp_dir().join(format!(
        "nanika-config-targeted-update-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let file = store.config_file();
    std::fs::write(
        &file,
        "{\n  // retained\n  \"formatVersion\": 1,\n  \"hotkey\": \"Ctrl+Space\"\n}\n",
    )
    .expect("config should exist");

    let value: serde_json::Value = store
        .update(
            &file,
            [("hotkey".to_owned(), serde_json::json!("Alt+Space"))],
            |_| Ok(()),
        )
        .expect("targeted update should succeed");

    assert_eq!(value["hotkey"], "Alt+Space");
    assert!(
        std::fs::read_to_string(&file)
            .expect("config should be readable")
            .contains("// retained")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn targeted_updates_create_missing_files() {
    let root = std::env::temp_dir().join(format!(
        "nanika-config-targeted-create-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let file = store.config_root().join("extensions/test/settings.jsonc");

    let value: serde_json::Value = store
        .update(
            &file,
            [("enabled".to_owned(), serde_json::json!(true))],
            |_| Ok(()),
        )
        .expect("targeted update should create the file");

    assert_eq!(value["enabled"], true);
    assert!(file.is_file());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rejected_targeted_updates_leave_the_original_untouched() {
    let root = std::env::temp_dir().join(format!(
        "nanika-config-targeted-validation-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(&root, root.join("config")).expect("store should open");
    let file = store.config_file();
    let original = "{\n  \"enabled\": false\n}\n";
    std::fs::write(&file, original).expect("config should exist");

    let result = store.update::<serde_json::Value>(
        &file,
        [("enabled".to_owned(), serde_json::json!(true))],
        |_| Err("rejected".to_owned()),
    );

    assert!(matches!(result, Err(ConfigError::Invalid(_))));
    assert_eq!(
        std::fs::read_to_string(&file).expect("config should remain readable"),
        original
    );
    let _ = std::fs::remove_dir_all(root);
}

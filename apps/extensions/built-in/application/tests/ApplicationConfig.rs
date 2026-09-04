use std::path::PathBuf;

use nanika_config::ConfigStore;
use nanika_protocol::{SettingUpdate, SettingValue};

use crate::ApplicationConfig;

#[test]
fn settings_require_absolute_discovery_paths() {
    let config = ApplicationConfig {
        format_version: 1,
        roots: vec![PathBuf::from("relative")],
        exclusions: Vec::new(),
    };
    assert!(config.validate().is_err());
}

#[test]
fn settings_path_stays_in_the_centralized_extension_config_tree() {
    let root = PathBuf::from("C:/nanika-config");
    assert_eq!(
        ApplicationConfig::path(&root),
        root.join("extensions/com.nanika.application/settings.jsonc")
    );
}

#[test]
fn settings_require_an_explicit_format_version() {
    let root = test_root("missing-version");
    let store = ConfigStore::open(root.join("data"), root.join("config"))
        .expect("config store should open");
    let path = ApplicationConfig::path(store.config_root());
    std::fs::create_dir_all(path.parent().expect("settings parent"))
        .expect("settings parent should exist");
    std::fs::write(&path, r#"{"roots":[],"exclusions":[]}"#).expect("settings should write");

    assert!(ApplicationConfig::load(&store).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn non_file_settings_paths_are_not_treated_as_missing() {
    let root = test_root("settings-directory");
    let store = ConfigStore::open(root.join("data"), root.join("config"))
        .expect("config store should open");
    let path = ApplicationConfig::path(store.config_root());
    std::fs::create_dir_all(&path).expect("settings directory should exist");

    assert!(ApplicationConfig::load(&store).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn shared_settings_updates_are_validated_and_persisted_by_the_extension() {
    let root = test_root("shared-update");
    let store = ConfigStore::open(root.join("data"), root.join("config"))
        .expect("config store should open");
    let config = ApplicationConfig::load(&store).expect("default config should load");
    config
        .settings()
        .validate()
        .expect("settings contribution should be valid");

    let updated = config
        .update(
            &store,
            vec![SettingUpdate {
                key: "roots".to_owned(),
                value: SettingValue::StringList {
                    values: vec![root.to_string_lossy().into_owned()],
                },
            }],
        )
        .expect("extension should persist its update");

    assert_eq!(updated.roots.as_slice(), std::slice::from_ref(&root));
    assert_eq!(
        ApplicationConfig::load(&store)
            .expect("persisted settings should load")
            .roots
            .as_slice(),
        std::slice::from_ref(&root)
    );
    let _ = std::fs::remove_dir_all(root);
}

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-config-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root should exist");
    root
}

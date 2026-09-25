use crate::{ConfigurationApplication, ExtensionConfigurationRegistry};
use nanika_config::{ConfigStore, ExtensionConfigurationFile};
use nanika_extension_package::ConfigurationContribution;
use std::sync::Arc;

fn fixture(
    name: &str,
    persistence: &str,
) -> (
    Arc<ExtensionConfigurationRegistry>,
    ConfigStore,
    std::path::PathBuf,
) {
    let root = std::env::temp_dir().join(format!("nanika-settings-{name}-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    let store = ConfigStore::open(root.join("machine"), root.join("configuration")).unwrap();
    let registry = Arc::new(ExtensionConfigurationRegistry::new(store.clone()));
    let contribution: ConfigurationContribution = serde_json::from_value(serde_json::json!({
        "title": "Fixture", "properties": {
            "enabled": {"type": "boolean", "title": "Enabled", "default": false, "persistence": persistence},
            "other": {"type": "boolean", "title": "Other", "default": true, "persistence": "beforeApply"},
            "windows": {"type": "boolean", "title": "Windows", "default": true, "persistence": "beforeApply", "platforms": ["windows"]},
            "macos": {"type": "boolean", "title": "macOS", "default": true, "persistence": "beforeApply", "platforms": ["macos"]}
        }
    })).unwrap();
    registry
        .register("test.extension", Some(&contribution))
        .unwrap();
    (registry, store, root)
}

#[test]
fn persistence_order_and_application_failure_preserve_the_right_facts() {
    for policy in ["beforeApply", "afterApply"] {
        for success in [true, false] {
            let (registry, store, root) = fixture(&format!("{policy}-{success}"), policy);
            let outcome = registry
                .prepare("test.extension", "enabled".into(), true.into())
                .unwrap()
                .run(|candidate, live| {
                    assert_eq!(candidate.values()["enabled"], true);
                    assert_eq!(live, policy == "afterApply");
                    assert_eq!(
                        store
                            .extension_configuration_file("test.extension")
                            .exists(),
                        policy == "beforeApply"
                    );
                    if success {
                        Ok(ConfigurationApplication::Applied)
                    } else {
                        Err("device rejected request".into())
                    }
                });
            assert_eq!(outcome.saved["enabled"], success || policy == "beforeApply");
            assert_eq!(outcome.error.is_some(), !success);
            assert_eq!(outcome.effective.is_some(), success);
            assert_eq!(outcome.values["other"], true);
            if success {
                assert_eq!(outcome.effective.unwrap()["enabled"], true);
            }
            if success || policy == "beforeApply" {
                let persisted: ExtensionConfigurationFile = store
                    .load(store.extension_configuration_file("test.extension"))
                    .unwrap();
                assert_eq!(persisted.values["enabled"], true);
                assert_eq!(persisted.values["macos"], true);
                assert_eq!(persisted.values["windows"], true);
            } else {
                assert!(
                    !store
                        .extension_configuration_file("test.extension")
                        .exists()
                );
            }
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}

#[test]
fn application_success_then_storage_failure_does_not_invent_rollback() {
    let (registry, store, root) = fixture("partial", "afterApply");
    let path = store.extension_configuration_file("test.extension");
    let outcome = registry
        .prepare("test.extension", "enabled".into(), true.into())
        .unwrap()
        .run(|_, _| {
            std::fs::create_dir_all(&path).unwrap();
            Ok(ConfigurationApplication::Applied)
        });
    assert_eq!(outcome.saved["enabled"], false);
    assert_eq!(outcome.values["enabled"], true);
    assert_eq!(outcome.effective.unwrap()["enabled"], true);
    assert!(
        outcome
            .error
            .unwrap()
            .contains("took effect but could not be saved")
    );
    // Editing an unrelated field starts from confirmed actual state, not a stale file.
    let next = registry
        .prepare("test.extension", "other".into(), false.into())
        .unwrap();
    assert_eq!(next.configuration.values()["enabled"], true);
    drop(next);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn persistence_failure_before_application_never_starts_the_task() {
    let (registry, store, root) = fixture("disk-failure", "beforeApply");
    std::fs::create_dir_all(store.extension_configuration_file("test.extension")).unwrap();
    let outcome = registry
        .prepare("test.extension", "enabled".into(), true.into())
        .unwrap()
        .run(|_, _| panic!("must not apply"));
    assert!(outcome.error.is_some());
    assert_eq!(outcome.saved["enabled"], false);
    assert!(
        registry
            .prepare("test.extension", "enabled".into(), true.into())
            .is_ok()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn admission_is_bounded_and_property_validation_preserves_hidden_values() {
    let (registry, store, root) = fixture("admission", "beforeApply");
    let hidden = if nanika_platform::target_platform() == "windows" {
        "macos"
    } else {
        "windows"
    };
    for (key, value) in [
        ("unknown", true.into()),
        (hidden, false.into()),
        ("enabled", "invalid".into()),
    ] {
        assert!(
            registry
                .prepare("test.extension", key.into(), value)
                .is_err()
        );
    }
    let operation = registry
        .prepare("test.extension", "enabled".into(), true.into())
        .unwrap();
    assert!(
        registry
            .prepare("test.extension", "other".into(), false.into())
            .is_err()
    );
    let outcome = operation.run(|_, _| Ok(ConfigurationApplication::Deferred));
    assert!(outcome.effective.is_none());
    assert!(outcome.error.is_none());
    let stored: ExtensionConfigurationFile = store
        .load(store.extension_configuration_file("test.extension"))
        .unwrap();
    assert_eq!(stored.values[hidden], true);
    assert_eq!(stored.values["enabled"], true);
    assert!(!outcome.values.contains_key(hidden));
    let operation = registry
        .prepare("test.extension", "enabled".into(), false.into())
        .unwrap();
    drop(operation);
    assert!(
        registry
            .prepare("test.extension", "enabled".into(), false.into())
            .is_ok()
    );
    registry.close();
    assert!(
        registry
            .prepare("test.extension", "enabled".into(), false.into())
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn apply_first_cannot_save_a_deferred_operation() {
    let (registry, store, root) = fixture("deferred", "afterApply");
    let outcome = registry
        .prepare("test.extension", "enabled".into(), true.into())
        .unwrap()
        .run(|_, _| Ok(ConfigurationApplication::Deferred));
    assert!(outcome.error.is_some());
    assert_eq!(outcome.saved["enabled"], false);
    assert!(
        !store
            .extension_configuration_file("test.extension")
            .exists()
    );
    std::fs::remove_dir_all(root).unwrap();
}

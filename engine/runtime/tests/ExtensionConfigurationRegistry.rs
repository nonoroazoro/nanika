use crate::{ConfigurationApplication, ExtensionConfigurationRegistry};
use nanika_config::{ConfigStore, ExtensionConfigurationFile};
use nanika_extension_package::ConfigurationContribution;
use std::sync::Arc;

fn fixture(
    name: &str,
    persistence: &str
) -> (
    Arc<ExtensionConfigurationRegistry>,
    ConfigStore,
    std::path::PathBuf
) {
    let root = std::env::temp_dir().join(format!("nanika-settings-{name}-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    let store = ConfigStore::open(root.join("machine"), root.join("configuration")).unwrap();
    let registry = Arc::new(ExtensionConfigurationRegistry::new(store.clone()));
    let contribution: ConfigurationContribution = serde_json::from_value(serde_json::json!({
        "title": "Fixture",
        "quickMenu": [{"id": "display", "title": "Display", "items": [{"kind": "toggle", "key": "enabled"}]}],
        "properties": {
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
    let cause = crate::OperationError::with_detail(
        crate::OperationErrorCode::ExtensionIo,
        "device rejected request"
    );
    for policy in ["beforeApply", "afterApply"] {
        let failure = if policy == "beforeApply" {
            crate::ConfigurationSaveError::SavedButNotApplied {
                cause: Box::new(cause.clone())
            }
        } else {
            crate::ConfigurationSaveError::UnconfirmedNotSaved {
                cause: Box::new(cause.clone())
            }
        };
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
                        Err(cause.clone())
                    }
                });
            assert_eq!(outcome.saved["enabled"], success || policy == "beforeApply");
            assert_eq!(outcome.error, (!success).then_some(failure.clone()));
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
        matches!(outcome.error, Some(crate::ConfigurationSaveError::AppliedButNotSaved { detail }) if !detail.is_empty())
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
    assert!(
        matches!(outcome.error, Some(crate::ConfigurationSaveError::SaveFailed { detail }) if !detail.is_empty())
    );
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
    let hidden = if nanika_platform::target_platform() == nanika_platform::TargetPlatform::Windows {
        "macos"
    } else {
        "windows"
    };
    for (key, value) in [
        ("unknown", true.into()),
        (hidden, false.into()),
        ("enabled", "invalid".into())
    ] {
        let error = registry
            .prepare("test.extension", key.into(), value)
            .err()
            .unwrap();
        let code = if key == "enabled" {
            crate::OperationErrorCode::InvalidConfiguration
        } else {
            crate::OperationErrorCode::ConfigurationPropertyUnavailable
        };
        assert!(
            matches!(error, crate::OperationError::Host { code: actual, .. } if actual == code)
        );
        assert!(
            !store
                .extension_configuration_file("test.extension")
                .exists()
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
    assert_eq!(
        outcome.error,
        Some(crate::ConfigurationSaveError::Unavailable)
    );
    assert_eq!(outcome.saved["enabled"], false);
    assert!(
        !store
            .extension_configuration_file("test.extension")
            .exists()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn menu_changes_share_settings_admission_and_reject_stale_values_and_unexposed_keys() {
    let (registry, _, root) = fixture("quick-menu", "afterApply");
    let revision = registry.snapshots()[0].revision;
    assert!(
        registry
            .prepare_menu("test.extension", "other".into(), false, revision)
            .is_err()
    );
    let operation = registry
        .prepare_menu("test.extension", "enabled".into(), true, revision)
        .unwrap();
    assert!(
        registry
            .prepare("test.extension", "other".into(), false.into())
            .is_err()
    );
    assert!(
        registry
            .prepare_menu("test.extension", "enabled".into(), true, revision)
            .is_err()
    );
    let outcome = operation.run(|_, live| {
        assert!(live);
        Ok(ConfigurationApplication::Applied)
    });
    assert_eq!(outcome.saved["enabled"], true);
    assert!(
        registry
            .prepare_menu("test.extension", "enabled".into(), false, revision)
            .is_err()
    );
    let outcome = registry
        .prepare_menu("test.extension", "enabled".into(), false, outcome.revision)
        .unwrap()
        .run(|_, _| {
            Err(crate::OperationError::new(
                crate::OperationErrorCode::ExtensionIo
            ))
        });
    assert!(outcome.error.is_some());
    assert_eq!(
        outcome.saved["enabled"], true,
        "failed afterApply writes preserve saved intent"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn integer_limits_preserve_values_across_reload() {
    let root = std::env::temp_dir().join(format!("nanika-integer-limit-{}", std::process::id()));
    let store = ConfigStore::open(root.join("machine"), root.join("configuration")).unwrap();
    let contribution: ConfigurationContribution = serde_json::from_value(serde_json::json!({
        "title":"Limits", "properties": {
            "count":{"type":"integerLimit","title":"Count","default":{"unlimited":false,"value":50},"minimum":1,"maximum":5000,"persistence":"beforeApply"},
            "age":{"type":"integerLimit","title":"Age","default":{"unlimited":false,"value":7},"persistence":"beforeApply"}
        }
    })).unwrap();
    let path = store.extension_configuration_file("test.limits");
    store
        .save(
            &path,
            &ExtensionConfigurationFile::new(std::collections::BTreeMap::from([
                (
                    "count".into(),
                    serde_json::json!({"unlimited":false,"value":200})
                ),
                (
                    "age".into(),
                    serde_json::json!({"unlimited":true,"value":7})
                )
            ]))
        )
        .unwrap();
    let registry = Arc::new(ExtensionConfigurationRegistry::new(store.clone()));
    let values = registry
        .register("test.limits", Some(&contribution))
        .unwrap();
    assert_eq!(
        values.values()["count"],
        serde_json::json!({"unlimited":false,"value":200})
    );
    assert_eq!(
        values.values()["age"],
        serde_json::json!({"unlimited":true,"value":7})
    );
    let stored = store.load::<ExtensionConfigurationFile>(&path).unwrap();
    assert_eq!(&stored.values, values.values());

    let outcome = registry
        .prepare(
            "test.limits",
            "count".into(),
            serde_json::json!({"unlimited":true,"value":200})
        )
        .unwrap()
        .run(|_, _| Ok(ConfigurationApplication::Applied));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let reloaded = Arc::new(ExtensionConfigurationRegistry::new(store.clone()));
    let values = reloaded
        .register("test.limits", Some(&contribution))
        .unwrap();
    assert_eq!(
        values.values()["count"],
        serde_json::json!({"unlimited":true,"value":200})
    );
    let outcome = reloaded
        .prepare(
            "test.limits",
            "count".into(),
            serde_json::json!({"unlimited":false,"value":200})
        )
        .unwrap()
        .run(|_, _| Ok(ConfigurationApplication::Applied));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(
        store
            .load::<ExtensionConfigurationFile>(&path)
            .unwrap()
            .values["count"],
        serde_json::json!({"unlimited":false,"value":200})
    );
    assert!(
        reloaded
            .prepare("test.limits", "count".into(), serde_json::Value::Null)
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_integer_limits_use_defaults_without_modifying_the_file() {
    let root = std::env::temp_dir().join(format!("nanika-invalid-limit-{}", std::process::id()));
    let store = ConfigStore::open(root.join("machine"), root.join("configuration")).unwrap();
    let contribution: ConfigurationContribution = serde_json::from_value(serde_json::json!({
        "title":"Limits", "properties": {
            "count":{"type":"integerLimit","title":"Count","default":{"unlimited":false,"value":50},"minimum":1,"persistence":"beforeApply"}
        }
    })).unwrap();
    let path = store.extension_configuration_file("test.limits");
    for invalid in [
        serde_json::json!(200),
        serde_json::Value::Null,
        serde_json::json!({"unlimited":false,"value":0}),
        serde_json::json!({"unlimited":true}),
        serde_json::json!({"unlimited":true,"value":50,"extra":true})
    ] {
        store
            .save(
                &path,
                &ExtensionConfigurationFile::new(std::collections::BTreeMap::from([(
                    "count".into(),
                    invalid.clone()
                )]))
            )
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        let registry = ExtensionConfigurationRegistry::new(store.clone());
        let values = registry
            .register("test.limits", Some(&contribution))
            .unwrap();
        assert_eq!(
            values.values()["count"],
            contribution.properties["count"].default
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn configuration_recovery_preserves_valid_options_and_saves_only_on_edit() {
    let (initial, store, root) = fixture("option-defaults", "beforeApply");
    let contribution = initial.snapshots()[0].contribution.clone();
    let path = store.extension_configuration_file("test.extension");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    for text in [
        r#"{"formatVersion":1,"values":{"enabled":"wrong","other":false}}"#,
        r#"{"formatVersion":1,"values":{"enabled":"wrong","other":false,"unknown":true}}"#,
        "{ malformed"
    ] {
        std::fs::write(&path, text).unwrap();
        let registry = Arc::new(ExtensionConfigurationRegistry::new(store.clone()));
        let values = registry
            .register("test.extension", Some(&contribution))
            .unwrap();
        assert_eq!(values.values()["enabled"], false);
        assert_eq!(values.values()["other"], text == "{ malformed");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        let outcome = registry
            .prepare("test.extension", "enabled".into(), true.into())
            .unwrap()
            .run(|_, _| Ok(ConfigurationApplication::Applied));
        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        let saved = store.load::<ExtensionConfigurationFile>(&path).unwrap();
        contribution.validate_values(&saved.values).unwrap();
        assert_eq!(saved.values["enabled"], true);
        assert_eq!(saved.values["other"], text == "{ malformed");
    }
    std::fs::remove_dir_all(root).unwrap();
}

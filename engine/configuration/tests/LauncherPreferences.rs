use nanika_config::{ConfigStore, LauncherPreferences, ThemePreference};

fn store(name: &str) -> (std::path::PathBuf, ConfigStore) {
    let root = std::env::temp_dir().join(format!(
        "nanika-host-preferences-{name}-{}",
        uuid::Uuid::new_v4()
    ));
    let store = ConfigStore::open(root.join("machine"), root.join("config")).unwrap();
    (root, store)
}

#[test]
fn host_preferences_work_without_any_extension_and_survive_reopen() {
    let (root, store) = store("zero-extensions");
    let mut preferences = LauncherPreferences::load(&store, "Ctrl+Alt+Space").unwrap();
    assert_eq!(preferences.theme, ThemePreference::System);
    assert!(preferences.hide_on_blur);
    preferences.launcher_shortcut = "Ctrl+Alt+KeyK".to_owned();
    preferences.theme = ThemePreference::Dark;
    preferences.hide_on_blur = false;
    preferences.save(&store).unwrap();
    let loaded = LauncherPreferences::load(&store, "Ctrl+Space").unwrap();
    assert_eq!(loaded.launcher_shortcut, preferences.launcher_shortcut);
    assert_eq!(loaded.theme, ThemePreference::Dark);
    assert!(!loaded.hide_on_blur);
    assert!(!store.extensions_file().exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn host_saves_preserve_comments_and_do_not_touch_extension_configuration() {
    let (root, store) = store("isolation");
    let extension = store.extension_configuration_file("test.extension");
    store
        .save(
            &extension,
            &serde_json::json!({"formatVersion": 1, "values": {"enabled": true}}),
        )
        .unwrap();
    let original_extension = std::fs::read(&extension).unwrap();
    std::fs::write(store.config_file(), "{\n// User theme preference\n\"formatVersion\":1,\"launcherShortcut\":\"Ctrl+Space\",\"theme\":\"system\",\"hideOnBlur\":true\n}\n").unwrap();
    let mut preferences = LauncherPreferences::load(&store, "Ctrl+Space").unwrap();
    preferences.theme = ThemePreference::Light;
    preferences.save(&store).unwrap();
    assert!(
        std::fs::read_to_string(store.config_file())
            .unwrap()
            .contains("// User theme preference")
    );
    assert_eq!(std::fs::read(extension).unwrap(), original_extension);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_preferences_cannot_replace_a_saved_file() {
    let (root, store) = store("invalid");
    let mut preferences = LauncherPreferences::load(&store, "Ctrl+Space").unwrap();
    preferences.save(&store).unwrap();
    let before = std::fs::read(store.config_file()).unwrap();
    preferences.launcher_shortcut = "\n".to_owned();
    assert!(preferences.save(&store).is_err());
    assert_eq!(std::fs::read(store.config_file()).unwrap(), before);
    preferences.launcher_shortcut = "Ctrl+Space".to_owned();
    preferences.format_version = 2;
    assert!(preferences.save(&store).is_err());
    assert_eq!(std::fs::read(store.config_file()).unwrap(), before);
    std::fs::remove_dir_all(root).unwrap();
}

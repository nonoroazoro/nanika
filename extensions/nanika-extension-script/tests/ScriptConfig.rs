use std::path::PathBuf;

use nanika_config::ConfigStore;
use nanika_extension_script::{ScriptConfig, ScriptEntry};
use nanika_protocol::SettingUpdate;

#[test]
fn script_settings_require_stable_ids_and_absolute_paths() {
    let config = ScriptConfig {
        format_version: 1,
        scripts: vec![ScriptEntry {
            id: "build-project".to_owned(),
            title: "Build project".to_owned(),
            aliases: vec!["build".to_owned()],
            interpreter: PathBuf::from("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe"),
            script: PathBuf::from("C:/scripts/build.ps1"),
            arguments: Vec::new(),
            working_directory: Some(PathBuf::from("C:/project")),
        }],
    };
    assert!(config.validate().is_ok());

    let mut invalid = config;
    invalid.scripts[0].script = PathBuf::from("relative.ps1");
    assert!(invalid.validate().is_err());
}

#[test]
fn script_settings_reject_more_than_the_protocol_candidate_limit() {
    let config = ScriptConfig {
        format_version: 1,
        scripts: (0..5_001)
            .map(|index| ScriptEntry {
                id: format!("script-{index}"),
                title: format!("Script {index}"),
                aliases: Vec::new(),
                interpreter: std::env::temp_dir().join("interpreter"),
                script: std::env::temp_dir().join("script"),
                arguments: Vec::new(),
                working_directory: None,
            })
            .collect(),
    };
    assert!(config.validate().is_err());
}

#[test]
fn record_table_updates_round_trip_through_extension_validation() {
    let root = std::env::temp_dir().join(format!(
        "nanika-script-settings-update-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(root.join("data"), root.join("config"))
        .expect("config store should open");
    let config = ScriptConfig {
        format_version: 1,
        scripts: vec![ScriptEntry {
            id: "build-project".to_owned(),
            title: "Build project".to_owned(),
            aliases: vec!["build".to_owned()],
            interpreter: root.join("pwsh.exe"),
            script: root.join("build.ps1"),
            arguments: vec!["--release".to_owned()],
            working_directory: Some(root.clone()),
        }],
    };
    let contribution = config.settings();
    contribution
        .validate()
        .expect("record contribution should validate");

    let updated = config
        .update(
            &store,
            vec![SettingUpdate {
                key: "scripts".to_owned(),
                value: contribution.fields[0].value.clone(),
            }],
        )
        .expect("record update should persist");

    assert_eq!(updated, config);
    assert_eq!(
        ScriptConfig::load(&store).expect("persisted scripts should load"),
        config
    );
    let _ = std::fs::remove_dir_all(root);
}

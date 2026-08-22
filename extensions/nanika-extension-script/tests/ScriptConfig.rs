use std::path::PathBuf;

use nanika_extension_script::{ScriptConfig, ScriptEntry};

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

use std::collections::BTreeMap;

use nanika_extension_script::ScriptConfig;
use nanika_protocol::ExtensionConfiguration;

#[test]
fn parses_host_configuration() {
    let configuration = ExtensionConfiguration::new(BTreeMap::from([(
        "script.entries".to_owned(),
        serde_json::json!([{
            "id": "build",
            "title": "Build project",
            "aliases": ["compile"],
            "interpreter": executable_path(),
            "script": script_path(),
            "arguments": ["--release"],
            "workingDirectory": working_directory()
        }]),
    )]));

    let config = ScriptConfig::from_configuration(&configuration)
        .expect("host configuration should be valid");

    assert_eq!(config.scripts.len(), 1);
    assert_eq!(config.scripts[0].id, "build");
}

#[test]
fn rejects_relative_executables() {
    let configuration = ExtensionConfiguration::new(BTreeMap::from([(
        "script.entries".to_owned(),
        serde_json::json!([{
            "id": "build",
            "title": "Build project",
            "aliases": [],
            "interpreter": "relative",
            "script": script_path(),
            "arguments": [],
            "workingDirectory": ""
        }]),
    )]));

    assert!(ScriptConfig::from_configuration(&configuration).is_err());
}

#[cfg(target_os = "macos")]
fn executable_path() -> &'static str {
    "/bin/sh"
}

#[cfg(windows)]
fn executable_path() -> &'static str {
    r"C:\Windows\System32\cmd.exe"
}

#[cfg(target_os = "macos")]
fn script_path() -> &'static str {
    "/tmp/build.sh"
}

#[cfg(windows)]
fn script_path() -> &'static str {
    r"C:\Temp\build.cmd"
}

#[cfg(target_os = "macos")]
fn working_directory() -> &'static str {
    "/tmp"
}

#[cfg(windows)]
fn working_directory() -> &'static str {
    r"C:\Temp"
}

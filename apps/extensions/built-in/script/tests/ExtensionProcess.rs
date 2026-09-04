use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nanika_extension_script::{ScriptConfig, ScriptEntry};
use nanika_protocol::{HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame};

#[test]
fn script_process_loads_central_settings_and_requests_host_launch() {
    let root = std::env::temp_dir().join(format!("nanika-script-process-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data_root = root.join("data");
    let config_root = root.join("config");
    write_settings(&config_root, &root);
    let mut child = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_nanika-extension-script")))
        .args([
            argument("data-root", &data_root),
            argument("cache-root", &root.join("cache")),
            argument("config-root", &config_root),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("script extension should spawn");
    let mut input = BufWriter::new(child.stdin.take().expect("child stdin"));
    let mut output = BufReader::new(child.stdout.take().expect("child stdout"));
    write_frame(
        &mut input,
        &Message::Initialize {
            request_id: "initialize".to_owned(),
            protocol: PROTOCOL_NAME.to_owned(),
        },
    )
    .expect("initialize should write");
    assert!(matches!(
        read_frame(&mut output).expect("initialize response"),
        Some(Message::Initialized { .. })
    ));
    write_frame(
        &mut input,
        &Message::GetSettings {
            request_id: "script-settings".to_owned(),
        },
    )
    .expect("settings request should write");
    let Some(Message::Settings { contribution, .. }) =
        read_frame(&mut output).expect("settings response")
    else {
        panic!("script extension should contribute settings");
    };
    assert_eq!(contribution.title, "Scripts");
    assert_eq!(contribution.fields.len(), 1);
    write_frame(
        &mut input,
        &Message::Query {
            request_id: "query".to_owned(),
            generation: 1,
            query: "build".to_owned(),
        },
    )
    .expect("query should write");
    let Some(Message::Snapshot { entries, .. }) = read_frame(&mut output).expect("query response")
    else {
        panic!("script extension should return a snapshot");
    };
    let entry = entries.into_iter().next().expect("script candidate");
    assert_eq!(entry.title, "Build project");
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "invoke".to_owned(),
            generation: 1,
            entry_id: entry.entry_id,
            action_id: entry.action_id,
        },
    )
    .expect("invoke should write");
    let Some(Message::HostRequest {
        request_id,
        parent_request_id,
        generation,
        ..
    }) = read_frame(&mut output).expect("host request")
    else {
        panic!("script extension should request host launch");
    };
    write_frame(
        &mut input,
        &Message::HostResponse {
            request_id,
            parent_request_id,
            generation,
            response: HostServiceResponse::Launched,
        },
    )
    .expect("host response should write");
    assert!(matches!(
        read_frame(&mut output).expect("action result"),
        Some(Message::Result { .. })
    ));
    write_frame(
        &mut input,
        &Message::Shutdown {
            request_id: "shutdown".to_owned(),
        },
    )
    .expect("shutdown should write");
    assert!(matches!(
        read_frame(&mut output).expect("shutdown response"),
        Some(Message::ShutdownAck { .. })
    ));
    assert!(child.wait().expect("child should exit").success());
    let _ = std::fs::remove_dir_all(root);
}

fn write_settings(config_root: &Path, root: &Path) {
    let config = ScriptConfig {
        format_version: 1,
        scripts: vec![ScriptEntry {
            id: "build-project".to_owned(),
            title: "Build project".to_owned(),
            aliases: vec!["build".to_owned()],
            interpreter: root.join("interpreter"),
            script: root.join("build.script"),
            arguments: vec!["--release".to_owned()],
            working_directory: Some(root.to_path_buf()),
        }],
    };
    let path = ScriptConfig::path(config_root);
    std::fs::create_dir_all(path.parent().expect("settings parent")).expect("settings directory");
    std::fs::write(
        path,
        serde_json::to_string_pretty(&config).expect("settings should serialize"),
    )
    .expect("settings should write");
}

fn argument(name: &str, path: &Path) -> String {
    format!("--{name}={}", path.display())
}

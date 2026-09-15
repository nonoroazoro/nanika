use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nanika_protocol::{
    ExtensionConfiguration, HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame,
};

#[test]
fn script_process_consumes_host_configuration_and_requests_host_launch() {
    let root = std::env::temp_dir().join(format!("nanika-script-process-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data_root = root.join("data");
    let mut child = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_nanika-extension-script")))
        .args([
            argument("data-root", &data_root),
            argument("cache-root", &root.join("cache")),
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
            configuration: script_configuration(&root),
        },
    )
    .expect("initialize should write");
    assert!(matches!(
        read_frame(&mut output).expect("initialize response"),
        Some(Message::Initialized { .. })
    ));
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

fn script_configuration(root: &Path) -> ExtensionConfiguration {
    ExtensionConfiguration::new(std::collections::BTreeMap::from([(
        "script.entries".to_owned(),
        serde_json::json!([{
            "id": "build-project",
            "title": "Build project",
            "aliases": ["build"],
            "interpreter": root.join("interpreter"),
            "script": root.join("build.script"),
            "arguments": ["--release"],
            "workingDirectory": root
        }]),
    )]))
}

fn argument(name: &str, path: &Path) -> String {
    format!("--{name}={}", path.display())
}

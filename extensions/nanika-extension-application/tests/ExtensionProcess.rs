use std::ffi::OsString;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nanika_extension_application::ApplicationConfig;
use nanika_protocol::{HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame};

#[test]
fn process_refreshes_a_configured_root_and_contributes_candidates() {
    let root = test_root();
    let data_root = root.join("data");
    let cache_root = root.join("cache");
    let config_root = root.join("config");
    let applications = root.join("applications");
    std::fs::create_dir_all(&applications).expect("application root should exist");
    create_executable(&applications.join("Nanika Sample.exe"));
    write_settings(&config_root, &applications);

    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-application"
    )))
    .args([
        argument("data-root", &data_root),
        argument("cache-root", &cache_root),
        argument("config-root", &config_root),
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("application extension should spawn");
    let mut input = BufWriter::new(child.stdin.take().expect("child stdin"));
    let mut output = BufReader::new(child.stdout.take().expect("child stdout"));
    write_frame(
        &mut input,
        &Message::Initialize {
            request_id: "initialize-application".to_owned(),
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
            request_id: "application-settings".to_owned(),
        },
    )
    .expect("settings request should write");
    let Some(Message::Settings { contribution, .. }) =
        read_frame(&mut output).expect("settings response")
    else {
        panic!("application extension should contribute settings");
    };
    assert_eq!(contribution.title, "Applications");
    assert_eq!(contribution.fields.len(), 2);
    write_frame(
        &mut input,
        &Message::Query {
            request_id: "startup-query".to_owned(),
            generation: 1,
            query: "nanika sample".to_owned(),
        },
    )
    .expect("startup query should write");
    let startup_entries = read_complete_snapshot(&mut output);
    assert!(
        startup_entries
            .iter()
            .any(|entry| entry.title == "Nanika Sample")
    );
    write_frame(
        &mut input,
        &Message::Refresh {
            request_id: "refresh-application".to_owned(),
            generation: 2,
        },
    )
    .expect("refresh should write");
    assert!(matches!(
        read_frame(&mut output).expect("refresh response"),
        Some(Message::Refreshed { generation: 2, .. })
    ));
    write_frame(
        &mut input,
        &Message::Query {
            request_id: "query-application".to_owned(),
            generation: 3,
            query: "nanika sample".to_owned(),
        },
    )
    .expect("query should write");
    let Some(Message::Snapshot { entries, .. }) = read_frame(&mut output).expect("query response")
    else {
        panic!("application extension should return a snapshot");
    };
    assert!(entries.iter().any(|entry| entry.title == "Nanika Sample"));
    let entry = entries
        .into_iter()
        .find(|entry| entry.title == "Nanika Sample")
        .expect("sample application candidate");
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "invoke-application".to_owned(),
            generation: 3,
            entry_id: entry.entry_id.clone(),
            action_id: entry.action_id.clone(),
        },
    )
    .expect("invoke should write");
    let Some(Message::HostRequest {
        request_id: service_request_id,
        parent_request_id,
        generation,
        ..
    }) = read_frame(&mut output).expect("host request")
    else {
        panic!("application extension should request host launch");
    };
    write_frame(
        &mut input,
        &Message::HostResponse {
            request_id: service_request_id,
            parent_request_id,
            generation,
            response: HostServiceResponse::Launched,
        },
    )
    .expect("host response should write");
    assert!(matches!(
        read_frame(&mut output).expect("invoke result"),
        Some(Message::Result { generation: 3, .. })
    ));
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "invoke-application-invalid".to_owned(),
            generation: 4,
            entry_id: entry.entry_id,
            action_id: entry.action_id,
        },
    )
    .expect("second invoke should write");
    let Some(Message::HostRequest {
        request_id: service_request_id,
        parent_request_id,
        generation,
        ..
    }) = read_frame(&mut output).expect("second host request")
    else {
        panic!("application extension should request host launch");
    };
    write_frame(
        &mut input,
        &Message::HostResponse {
            request_id: service_request_id,
            parent_request_id,
            generation: generation + 1,
            response: HostServiceResponse::Launched,
        },
    )
    .expect("invalid host response should write");
    assert!(matches!(
        read_frame(&mut output).expect("invalid invoke result"),
        Some(Message::Error {
            request_id: Some(request_id),
            code,
            ..
        }) if request_id == "invoke-application-invalid" && code == "invalid_host_response"
    ));
    write_frame(
        &mut input,
        &Message::Shutdown {
            request_id: "shutdown-application".to_owned(),
        },
    )
    .expect("shutdown should write");
    assert!(matches!(
        read_frame(&mut output).expect("shutdown response"),
        Some(Message::ShutdownAck { .. })
    ));
    drop(input);
    assert!(child.wait().expect("child should exit").success());
    let _ = std::fs::remove_dir_all(root);
}

fn read_complete_snapshot(output: &mut impl std::io::Read) -> Vec<nanika_protocol::Candidate> {
    loop {
        let Some(Message::Snapshot {
            complete, entries, ..
        }) = read_frame(output).expect("startup query response")
        else {
            panic!("application extension should return a startup snapshot");
        };
        if complete {
            return entries;
        }
    }
}

fn write_settings(config_root: &Path, application_root: &Path) {
    let directory = config_root.join("extensions/com.nanika.application");
    std::fs::create_dir_all(&directory).expect("settings directory should exist");
    let config = ApplicationConfig {
        format_version: 1,
        roots: vec![application_root.to_path_buf()],
        exclusions: ApplicationConfig::standard_roots().expect("standard roots"),
    };
    std::fs::write(
        directory.join("settings.jsonc"),
        serde_json::to_string_pretty(&config).expect("settings should serialize"),
    )
    .expect("settings should write");
}

fn argument(name: &str, value: &Path) -> OsString {
    OsString::from(format!("--{name}={}", value.display()))
}

fn create_executable(target: &Path) {
    let source = std::env::current_exe().expect("test executable path");
    std::fs::hard_link(&source, target)
        .or_else(|_| std::fs::copy(&source, target).map(|_| ()))
        .expect("test executable should exist");
}

fn test_root() -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("nanika-application-process-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root should exist");
    root
}

use std::ffi::OsString;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nanika_extension_application::ApplicationConfig;
use nanika_protocol::{
    ExtensionConfiguration, HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame,
};

#[test]
fn process_refreshes_a_configured_root_and_contributes_candidates() {
    let root = test_root("refresh");
    let data_root = root.join("data");
    let cache_root = root.join("cache");
    let applications = root.join("applications");
    std::fs::create_dir_all(&applications).expect("application root should exist");
    create_application_fixture(&applications);

    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-application"
    )))
    .args([
        argument("data-root", &data_root),
        argument("cache-root", &cache_root),
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
            configuration: application_configuration(&applications),
        },
    )
    .expect("initialize should write");
    assert!(matches!(
        read_response(&mut output, "initialize response"),
        Some(Message::Initialized { .. })
    ));
    query_until_candidate(
        &mut input,
        &mut output,
        "startup-query",
        1,
        "nanika sample",
        "Nanika Sample",
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
        read_response(&mut output, "refresh response"),
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
    let Some(Message::Snapshot { entries, .. }) = read_response(&mut output, "query response")
    else {
        panic!("application extension should return a snapshot");
    };
    assert!(entries.iter().any(|entry| entry.title == "Nanika Sample"));
    let entry = entries
        .into_iter()
        .find(|entry| entry.title == "Nanika Sample")
        .expect("sample application candidate");
    let icon_key = entry
        .icon
        .as_ref()
        .map(|icon| icon.key())
        .expect("application candidate should reference its icon");
    assert!(
        cache_root
            .join("icons")
            .join("com.nanika.application")
            .join(icon_key)
            .join("32.png")
            .is_file()
    );
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
    }) = read_response(&mut output, "host request")
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
        read_response(&mut output, "invoke result"),
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
    }) = read_response(&mut output, "second host request")
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
        read_response(&mut output, "invalid invoke result"),
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
        read_response(&mut output, "shutdown response"),
        Some(Message::ShutdownAck { .. })
    ));
    drop(input);
    assert!(child.wait().expect("child should exit").success());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn process_keeps_search_available_when_startup_icon_cache_fails() {
    let root = test_root("startup-cache-failure");
    let data_root = root.join("data");
    let cache_root = root.join("cache");
    let applications = root.join("applications");
    std::fs::create_dir_all(&applications).expect("application root should exist");
    create_application_fixture(&applications);
    std::fs::create_dir_all(cache_root.join("icons")).expect("icon parent should exist");
    std::fs::write(cache_root.join("icons/com.nanika.application"), [])
        .expect("invalid icon root should exist");

    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-application"
    )))
    .args([
        argument("data-root", &data_root),
        argument("cache-root", &cache_root),
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
            request_id: "initialize-application-failure".to_owned(),
            protocol: PROTOCOL_NAME.to_owned(),
            configuration: application_configuration(&applications),
        },
    )
    .expect("initialize should write");
    assert!(matches!(
        read_response(&mut output, "initialize response"),
        Some(Message::Initialized { .. })
    ));
    let entries = query_until_candidate(
        &mut input,
        &mut output,
        "startup-query-without-icons",
        1,
        "nanika sample",
        "Nanika Sample",
    );
    let entry = entries
        .iter()
        .find(|entry| entry.title == "Nanika Sample")
        .expect("search should remain available");
    assert!(entry.icon.is_none());
    write_frame(
        &mut input,
        &Message::Query {
            request_id: "cleared-query-without-icons".to_owned(),
            generation: 2,
            query: String::new(),
        },
    )
    .expect("cleared query should write");
    let cleared_entries = read_complete_snapshot(&mut output);
    assert!(
        cleared_entries
            .iter()
            .any(|entry| entry.title == "Nanika Sample")
    );
    write_frame(
        &mut input,
        &Message::Shutdown {
            request_id: "shutdown-application-failure".to_owned(),
        },
    )
    .expect("shutdown should write");
    loop {
        if matches!(
            read_response(&mut output, "shutdown response"),
            Some(Message::ShutdownAck { .. })
        ) {
            break;
        }
    }
    drop(input);
    assert!(child.wait().expect("child should exit").success());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn configuration_acknowledgement_waits_for_updated_candidates() {
    let root = test_root("configuration-acknowledgement");
    let data_root = root.join("data");
    let cache_root = root.join("cache");
    let initial_applications = root.join("initial-applications");
    let updated_applications = root.join("updated-applications");
    std::fs::create_dir_all(&initial_applications).expect("initial application root should exist");
    std::fs::create_dir_all(&updated_applications).expect("updated application root should exist");
    create_application_fixture(&updated_applications);

    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-application"
    )))
    .args([
        argument("data-root", &data_root),
        argument("cache-root", &cache_root),
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
            request_id: "initialize-configuration".to_owned(),
            protocol: PROTOCOL_NAME.to_owned(),
            configuration: application_configuration(&initial_applications),
        },
    )
    .expect("initialize should write");
    assert!(matches!(
        read_response(&mut output, "initialize response"),
        Some(Message::Initialized { .. })
    ));

    write_frame(
        &mut input,
        &Message::ConfigurationChanged {
            request_id: "change-configuration".to_owned(),
            configuration: application_configuration(&updated_applications),
        },
    )
    .expect("configuration change should write");
    assert!(matches!(
        read_response(&mut output, "configuration response"),
        Some(Message::ConfigurationApplied { request_id })
            if request_id == "change-configuration"
    ));

    write_frame(
        &mut input,
        &Message::Query {
            request_id: "query-updated-configuration".to_owned(),
            generation: 2,
            query: "nanika sample".to_owned(),
        },
    )
    .expect("query should write");
    let updated = read_complete_snapshot(&mut output);
    assert!(updated.iter().any(|entry| entry.title == "Nanika Sample"));
    prepare_entries(&mut input, 2, &updated);

    write_frame(
        &mut input,
        &Message::Shutdown {
            request_id: "shutdown-configuration".to_owned(),
        },
    )
    .expect("shutdown should write");
    assert!(matches!(
        read_response(&mut output, "shutdown response"),
        Some(Message::ShutdownAck { .. })
    ));
    drop(input);
    assert!(child.wait().expect("child should exit").success());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

fn read_complete_snapshot(output: &mut impl std::io::Read) -> Vec<nanika_protocol::Candidate> {
    loop {
        match read_frame(output).expect("query response") {
            Some(Message::Snapshot {
                complete: true,
                entries,
                ..
            }) => return entries,
            Some(Message::Snapshot { .. } | Message::CandidatesChanged) => {}
            message => panic!("application extension should return a snapshot, got {message:?}"),
        }
    }
}

fn query_until_candidate(
    input: &mut impl std::io::Write,
    output: &mut impl std::io::Read,
    request_id: &str,
    generation: u64,
    query: &str,
    title: &str,
) -> Vec<nanika_protocol::Candidate> {
    loop {
        write_frame(
            &mut *input,
            &Message::Query {
                request_id: request_id.to_owned(),
                generation,
                query: query.to_owned(),
            },
        )
        .expect("query should write");
        let entries = read_complete_snapshot(output);
        if entries.iter().any(|entry| entry.title == title) {
            prepare_entries(&mut *input, generation, &entries);
            return entries;
        }
        loop {
            if matches!(
                read_frame(&mut *output).expect("candidate change"),
                Some(Message::CandidatesChanged)
            ) {
                break;
            }
        }
    }
}

fn prepare_entries(
    input: &mut impl std::io::Write,
    generation: u64,
    entries: &[nanika_protocol::Candidate],
) {
    write_frame(
        input,
        &Message::PrepareEntries {
            generation,
            entry_ids: entries
                .iter()
                .take(10)
                .map(|entry| entry.entry_id.clone())
                .collect(),
        },
    )
    .expect("visible entry preparation should write");
}

fn read_response(output: &mut impl std::io::Read, context: &str) -> Option<Message> {
    loop {
        match read_frame(output).expect(context) {
            Some(Message::CandidatesChanged) => {}
            response => return response,
        }
    }
}

fn application_configuration(application_root: &Path) -> ExtensionConfiguration {
    ExtensionConfiguration::new(std::collections::BTreeMap::from([
        (
            "application.exclusions".to_owned(),
            serde_json::json!(ApplicationConfig::standard_roots().expect("standard roots")),
        ),
        (
            "application.roots".to_owned(),
            serde_json::json!([application_root]),
        ),
    ]))
}

fn argument(name: &str, value: &Path) -> OsString {
    OsString::from(format!("--{name}={}", value.display()))
}

#[cfg(windows)]
fn create_application_fixture(root: &Path) {
    create_executable(&root.join("Nanika Sample.exe"));
}

#[cfg(target_os = "macos")]
fn create_application_fixture(root: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let bundle = root.join("Nanika Sample.app/Contents");
    let executable = bundle.join("MacOS/nanika-sample");
    std::fs::create_dir_all(
        executable
            .parent()
            .expect("application executable should have a parent"),
    )
    .expect("application executable directory should exist");
    std::fs::write(
        bundle.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>CFBundleDisplayName</key>
  <string>Nanika Sample</string>
  <key>CFBundleExecutable</key>
  <string>nanika-sample</string>
  <key>CFBundleIdentifier</key>
  <string>com.nanika.test.sample</string>
</dict>
</plist>
"#,
    )
    .expect("application property list should exist");
    create_executable(&executable);
    let mut permissions = executable
        .metadata()
        .expect("application executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(executable, permissions)
        .expect("application executable should be executable");
}

#[cfg(not(any(windows, target_os = "macos")))]
fn create_application_fixture(root: &Path) {
    create_executable(&root.join("Nanika Sample.exe"));
}

fn create_executable(target: &Path) {
    let source = std::env::current_exe().expect("test executable path");
    std::fs::hard_link(&source, target)
        .or_else(|_| std::fs::copy(&source, target).map(|_| ()))
        .expect("test executable should exist");
}

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-process-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root should exist");
    root
}

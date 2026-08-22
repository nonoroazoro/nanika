use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nanika_extension_clipboard::{ClipboardDatabase, ClipboardEntry, RuntimePaths};
use nanika_protocol::{
    ClipboardContent, HostServiceRequest, HostServiceResponse, Message, PROTOCOL_NAME, read_frame,
    write_frame,
};

#[test]
fn clipboard_process_contributes_persisted_history_and_restores_through_the_host() {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-process-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let paths =
        RuntimePaths::parse([format!("--data-root={}", root.display())]).expect("runtime paths");
    let database = ClipboardDatabase::open(paths.database_path()).expect("database");
    database
        .upsert(&ClipboardEntry {
            entry_id: "clipboard.persisted".to_owned(),
            content_hash: "persisted".to_owned(),
            title: "Persisted clipboard text".to_owned(),
            content: ClipboardContent::Text {
                value: "Nanika clipboard payload".to_owned(),
            },
            byte_size: 24,
            captured_at: unix_timestamp(),
            pinned: false,
        })
        .expect("history should persist");
    drop(database);
    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-clipboard"
    )))
    .args([
        argument("data-root", &root),
        argument("cache-root", &root.join("cache")),
        argument("config-root", &root.join("config")),
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .expect("clipboard extension should spawn");
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
        &Message::Query {
            request_id: "query".to_owned(),
            generation: 1,
            query: "persisted".to_owned(),
        },
    )
    .expect("query should write");
    let Some(Message::Snapshot { entries, .. }) = read_frame(&mut output).expect("query response")
    else {
        panic!("clipboard extension should return a snapshot");
    };
    let entry = entries
        .into_iter()
        .find(|entry| entry.entry_id == "clipboard.persisted")
        .expect("persisted candidate");
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
        request:
            HostServiceRequest::WriteClipboard {
                content: ClipboardContent::Text { value },
            },
    }) = read_frame(&mut output).expect("host request")
    else {
        panic!("clipboard extension should request a host clipboard write");
    };
    assert_eq!(value, "Nanika clipboard payload");
    write_frame(
        &mut input,
        &Message::HostResponse {
            request_id,
            parent_request_id,
            generation,
            response: HostServiceResponse::ClipboardWritten,
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

fn argument(name: &str, path: &Path) -> String {
    format!("--{name}={}", path.display())
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

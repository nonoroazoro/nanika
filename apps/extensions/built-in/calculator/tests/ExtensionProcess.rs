use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use nanika_protocol::{
    ClipboardContent, HostServiceRequest, HostServiceResponse, Message, PROTOCOL_NAME, read_frame,
    write_frame,
};

#[test]
fn calculator_process_contributes_and_copies_through_the_host() {
    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-calculator"
    )))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .expect("calculator extension should spawn");
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
            query: "6 * 7".to_owned(),
        },
    )
    .expect("query should write");
    let Some(Message::Snapshot { entries, .. }) = read_frame(&mut output).expect("query response")
    else {
        panic!("calculator extension should return a snapshot");
    };
    let entry = entries.into_iter().next().expect("calculator candidate");
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
        panic!("calculator should request a clipboard write");
    };
    assert_eq!(value, "42");
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
}

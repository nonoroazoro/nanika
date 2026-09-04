use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use nanika_protocol::{HostServiceResponse, Message, PROTOCOL_NAME, read_frame, write_frame};

#[test]
fn command_process_contributes_and_requests_an_explicit_shell_launch() {
    let mut child = Command::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nanika-extension-command"
    )))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .expect("command extension should spawn");
    let mut input = BufWriter::new(child.stdin.take().expect("child stdin"));
    let mut output = BufReader::new(child.stdout.take().expect("child stdout"));
    initialize(&mut input, &mut output);
    write_frame(
        &mut input,
        &Message::Query {
            request_id: "query-command".to_owned(),
            generation: 1,
            query: "> echo nanika".to_owned(),
        },
    )
    .expect("query should write");
    let Some(Message::Snapshot { entries, .. }) = read_frame(&mut output).expect("query response")
    else {
        panic!("command extension should return a snapshot");
    };
    let entry = entries.into_iter().next().expect("command candidate");
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "invoke-command".to_owned(),
            generation: 1,
            entry_id: entry.entry_id,
            action_id: entry.action_id,
        },
    )
    .expect("invoke should write");
    complete_host_request(&mut input, &mut output, HostServiceResponse::Launched);
    shutdown(&mut child, &mut input, &mut output);
}

fn initialize(input: &mut impl std::io::Write, output: &mut impl std::io::Read) {
    write_frame(
        input,
        &Message::Initialize {
            request_id: "initialize".to_owned(),
            protocol: PROTOCOL_NAME.to_owned(),
        },
    )
    .expect("initialize should write");
    assert!(matches!(
        read_frame(output).expect("initialize response"),
        Some(Message::Initialized { .. })
    ));
}

fn complete_host_request(
    input: &mut impl std::io::Write,
    output: &mut impl std::io::Read,
    response: HostServiceResponse,
) {
    let Some(Message::HostRequest {
        request_id,
        parent_request_id,
        generation,
        ..
    }) = read_frame(output).expect("host request")
    else {
        panic!("extension should request a host service");
    };
    write_frame(
        input,
        &Message::HostResponse {
            request_id,
            parent_request_id,
            generation,
            response,
        },
    )
    .expect("host response should write");
    assert!(matches!(
        read_frame(output).expect("action result"),
        Some(Message::Result { .. })
    ));
}

fn shutdown(
    child: &mut std::process::Child,
    input: &mut impl std::io::Write,
    output: &mut impl std::io::Read,
) {
    write_frame(
        input,
        &Message::Shutdown {
            request_id: "shutdown".to_owned(),
        },
    )
    .expect("shutdown should write");
    assert!(matches!(
        read_frame(output).expect("shutdown response"),
        Some(Message::ShutdownAck { .. })
    ));
    assert!(child.wait().expect("child should exit").success());
}

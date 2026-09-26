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
            configuration: Default::default(),
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
            incremental: false,
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
            response: HostServiceResponse::ClipboardWritten { revision: 1 },
        },
    )
    .expect("host response should write");
    assert!(matches!(
        read_frame(&mut output).expect("action result"),
        Some(Message::Result { .. })
    ));
    drop(input);
    assert!(child.wait().expect("child should exit").success());
}

#[test]
fn explicit_cancellation_interrupts_evaluation_and_allows_the_next_query() {
    use std::sync::mpsc;
    use std::time::Duration;
    let mut child = Command::new(env!("CARGO_BIN_EXE_nanika-extension-calculator"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = BufWriter::new(child.stdin.take().unwrap());
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        while let Ok(Some(message)) = read_frame(&mut output) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    let result = (|| -> Result<(), String> {
        write_frame(
            &mut input,
            &Message::Initialize {
                request_id: "init".to_owned(),
                protocol: PROTOCOL_NAME.to_owned(),
                configuration: Default::default(),
            },
        )
        .unwrap();
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        write_frame(
            &mut input,
            &Message::Query {
                incremental: false,
                request_id: "slow".to_owned(),
                generation: 1,
                query: "100000!".to_owned(),
            },
        )
        .unwrap();
        // Test-only observation interval verifies cancellation interrupts active work.
        if receiver.recv_timeout(Duration::from_millis(100)).is_ok() {
            return Err("expected the expensive evaluation to remain active".to_owned());
        }
        write_frame(
            &mut input,
            &Message::Cancel {
                request_id: "slow".to_owned(),
                generation: 1,
            },
        )
        .unwrap();
        write_frame(
            &mut input,
            &Message::Query {
                incremental: false,
                request_id: "latest".to_owned(),
                generation: 2,
                query: "1+1".to_owned(),
            },
        )
        .unwrap();
        let cancelled = receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        if !matches!(cancelled, Message::Error { request_id: Some(id), code, .. } if id == "slow" && code == "cancelled")
        {
            return Err("old query must report explicit cancellation".to_owned());
        }
        let latest = receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        if !matches!(latest, Message::Snapshot { request_id, entries, complete: true, .. } if request_id == "latest" && entries.len() == 1 && entries[0].title == "= 2")
        {
            return Err("latest query must complete after cancellation".to_owned());
        }
        Ok(())
    })();
    let _ = child.kill();
    child.wait().unwrap();
    reader.join().unwrap();
    result.unwrap();
}

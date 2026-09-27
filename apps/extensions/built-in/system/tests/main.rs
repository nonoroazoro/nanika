use nanika_protocol::{
    COMMAND_EXECUTE_ACTION_ID, HostServiceRequest, HostServiceResponse, Message, NavigationEffect,
    PROTOCOL_NAME, SystemAction, read_frame, write_frame,
};
use std::io::{BufReader, BufWriter};
use std::process::{Command, Stdio};

#[test]
fn actions_use_correlated_host_requests_preserve_failures_and_do_not_cancel_accepted_work() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nanika-extension-system"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = BufWriter::new(child.stdin.take().unwrap());
    let mut output = BufReader::new(child.stdout.take().unwrap());
    write_frame(
        &mut input,
        &Message::Initialize {
            request_id: "init".into(),
            protocol: PROTOCOL_NAME.into(),
            configuration: Default::default(),
        },
    )
    .unwrap();
    assert!(matches!(
        read_frame(&mut output).unwrap(),
        Some(Message::Initialized { .. })
    ));
    for (entry_id, action) in [
        ("lock", SystemAction::Lock),
        ("sleep", SystemAction::Sleep),
        ("displays", SystemAction::TurnOffDisplays),
        ("logout", SystemAction::LogOut),
        ("restart", SystemAction::Restart),
        ("shutdown", SystemAction::ShutDown),
        ("trash.open", SystemAction::OpenTrash),
        ("trash.empty", SystemAction::EmptyTrash),
    ] {
        write_frame(
            &mut input,
            &Message::Invoke {
                request_id: "invoke".into(),
                generation: 7,
                entry_id: entry_id.into(),
                action_id: COMMAND_EXECUTE_ACTION_ID.into(),
            },
        )
        .unwrap();
        let Some(Message::HostRequest {
            request_id,
            parent_request_id,
            generation,
            request,
        }) = read_frame(&mut output).unwrap()
        else {
            panic!("host request");
        };
        assert_eq!(request, HostServiceRequest::SystemAction { action });
        assert_eq!(parent_request_id, "invoke");
        assert_eq!(generation, 7);
        write_frame(
            &mut input,
            &Message::Cancel {
                request_id: "invoke".into(),
                generation,
            },
        )
        .unwrap();
        write_frame(
            &mut input,
            &Message::Invoke {
                request_id: "second".into(),
                generation,
                entry_id: "lock".into(),
                action_id: COMMAND_EXECUTE_ACTION_ID.into(),
            },
        )
        .unwrap();
        assert!(
            matches!(read_frame(&mut output).unwrap(), Some(Message::Error { code, .. }) if code == "busy")
        );
        write_frame(
            &mut input,
            &Message::HostResponse {
                request_id,
                parent_request_id,
                generation,
                response: HostServiceResponse::SystemActionSubmitted,
            },
        )
        .unwrap();
        let Some(Message::Result {
            request_id,
            generation,
            effect,
        }) = read_frame(&mut output).unwrap()
        else {
            panic!("result");
        };
        assert_eq!(request_id, "invoke");
        assert_eq!(generation, 7);
        assert_eq!(effect, NavigationEffect::Dismiss);
    }
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "failure".into(),
            generation: 8,
            entry_id: "sleep".into(),
            action_id: COMMAND_EXECUTE_ACTION_ID.into(),
        },
    )
    .unwrap();
    let Some(Message::HostRequest { request_id, .. }) = read_frame(&mut output).unwrap() else {
        panic!("host request");
    };
    write_frame(
        &mut input,
        &Message::Error {
            request_id: Some(request_id),
            code: "denied".into(),
            message: "OS permission denied".into(),
        },
    )
    .unwrap();
    assert!(
        matches!(read_frame(&mut output).unwrap(), Some(Message::Error { request_id: Some(id), code, message }) if id == "failure" && code == "denied" && message == "OS permission denied")
    );
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "unknown".into(),
            generation: 9,
            entry_id: "codex".into(),
            action_id: COMMAND_EXECUTE_ACTION_ID.into(),
        },
    )
    .unwrap();
    assert!(
        matches!(read_frame(&mut output).unwrap(), Some(Message::Error { code, .. }) if code == "unknown_action")
    );
    drop(input);
    assert!(child.wait().unwrap().success());
}

#[test]
fn mismatched_response_cannot_complete_an_action() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nanika-extension-system"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = BufWriter::new(child.stdin.take().unwrap());
    let mut output = BufReader::new(child.stdout.take().unwrap());
    write_frame(
        &mut input,
        &Message::Initialize {
            request_id: "init".into(),
            protocol: PROTOCOL_NAME.into(),
            configuration: Default::default(),
        },
    )
    .unwrap();
    read_frame(&mut output).unwrap();
    write_frame(
        &mut input,
        &Message::Invoke {
            request_id: "invoke".into(),
            generation: 7,
            entry_id: "lock".into(),
            action_id: COMMAND_EXECUTE_ACTION_ID.into(),
        },
    )
    .unwrap();
    let Some(Message::HostRequest {
        request_id,
        parent_request_id,
        ..
    }) = read_frame(&mut output).unwrap()
    else {
        panic!("request");
    };
    write_frame(
        &mut input,
        &Message::HostResponse {
            request_id,
            parent_request_id,
            generation: 8,
            response: HostServiceResponse::SystemActionSubmitted,
        },
    )
    .unwrap();
    assert!(read_frame(&mut output).unwrap().is_none());
    drop(input);
    assert!(!child.wait().unwrap().success());
}

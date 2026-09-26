use std::io::{Cursor, Read};

use nanika_protocol::{FrameError, Message, PROTOCOL_NAME, read_frame, write_frame};

#[test]
fn round_trips_a_message() {
    let message = Message::Initialize {
        request_id: "request-1".to_owned(),
        protocol: PROTOCOL_NAME.to_owned(),
        configuration: Default::default(),
    };
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &message).expect("frame should be written");
    let decoded = read_frame(&mut Cursor::new(bytes))
        .expect("frame should be read")
        .expect("frame should exist");
    assert_eq!(decoded, message);
}

#[test]
fn clean_eof_is_not_an_error() {
    assert_eq!(
        read_frame(&mut Cursor::new([])).expect("read should succeed"),
        None
    );
}

#[test]
fn truncated_length_is_rejected() {
    let error = read_frame(&mut Cursor::new([1, 2])).expect_err("truncated length should fail");
    assert!(
        matches!(error, FrameError::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof)
    );
}

#[test]
fn large_configuration_round_trips_for_initialization_and_live_changes() {
    let configuration = nanika_protocol::ExtensionConfiguration::new(
        [(
            "entries".to_owned(),
            serde_json::json!(vec!["x".repeat(4000); 2400]),
        )]
        .into(),
    );
    for message in [
        Message::Initialize {
            request_id: "initialize".into(),
            protocol: PROTOCOL_NAME.into(),
            configuration: configuration.clone(),
        },
        Message::ConfigurationChanged {
            request_id: "change".into(),
            configuration: configuration.clone(),
        },
    ] {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &message).unwrap();
        assert!(bytes.len() > (8 * 1024 * 1024));
        let mut reader = Cursor::new(bytes);
        assert_eq!(read_frame(&mut reader).unwrap(), Some(message));
        assert_eq!(read_frame(&mut reader).unwrap(), None);
    }
}

#[test]
fn large_snapshot_round_trips_without_losing_the_following_frame() {
    let message = Message::Snapshot {
        request_id: "large-catalog".into(),
        generation: 1,
        complete: true,
        replace: true,
        removed: Vec::new(),
        entries: (0..6001)
            .map(|index| nanika_protocol::Candidate {
                kind: nanika_protocol::CandidateKind::Action,
                entry_id: format!("entry-{index}"),
                title: "x".repeat(1500),
                subtitle: None,
                action_id: "open".into(),
                actions: vec![nanika_protocol::Action::primary("open", "Open")],
                aliases: Vec::new(),
                icon: None,
            })
            .collect(),
    };
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &message).unwrap();
    assert!(bytes.len() > 8 * 1024 * 1024);
    write_frame(&mut bytes, &Message::CandidatesChanged).unwrap();
    let mut reader = Cursor::new(bytes);
    assert_eq!(read_frame(&mut reader).unwrap(), Some(message));
    assert_eq!(
        read_frame(&mut reader).unwrap(),
        Some(Message::CandidatesChanged)
    );
    assert_eq!(read_frame(&mut reader).unwrap(), None);
}

#[test]
fn unfulfilled_lengths_are_rejected_without_reserving_the_declared_payload() {
    for length in [8 * 1024 * 1024 + 1, u32::MAX] {
        let header = length.to_le_bytes();
        assert!(
            matches!(read_frame(&mut Cursor::new(header)), Err(FrameError::Io(error))
            if error.kind() == std::io::ErrorKind::UnexpectedEof)
        );
    }
}

#[test]
fn fragmented_messages_preserve_frame_boundaries() {
    let message = Message::Error {
        request_id: None,
        code: "fixture".into(),
        message: "caf\u{e9}".into(),
    };
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &message).unwrap();
    write_frame(&mut bytes, &Message::CandidatesChanged).unwrap();
    for split in 0..=bytes.len() {
        let mut reader = Cursor::new(&bytes[..split]).chain(Cursor::new(&bytes[split..]));
        assert_eq!(read_frame(&mut reader).unwrap(), Some(message.clone()));
        assert_eq!(
            read_frame(&mut reader).unwrap(),
            Some(Message::CandidatesChanged)
        );
        assert_eq!(read_frame(&mut reader).unwrap(), None);
    }
}

#[test]
fn invalid_payloads_do_not_consume_the_next_frame() {
    for payload in [
        b"".as_slice(),
        b"{",
        &[0xff],
        br#"{"type":"candidatesChanged"} trailing"#,
    ] {
        let mut bytes = (payload.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(payload);
        write_frame(&mut bytes, &Message::CandidatesChanged).unwrap();
        let mut reader = Cursor::new(bytes);
        assert!(matches!(read_frame(&mut reader), Err(FrameError::Json(_))));
        assert_eq!(
            read_frame(&mut reader).unwrap(),
            Some(Message::CandidatesChanged)
        );
    }
}

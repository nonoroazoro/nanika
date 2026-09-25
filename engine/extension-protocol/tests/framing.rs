use std::io::Cursor;

use nanika_protocol::{
    FrameError, MAX_EXTENSION_FRAME_BYTES, Message, PROTOCOL_NAME, read_extension_frame,
    read_host_frame, write_extension_frame, write_host_frame,
};

#[test]
fn round_trips_a_message() {
    let message = Message::Initialize {
        request_id: "request-1".to_owned(),
        protocol: PROTOCOL_NAME.to_owned(),
        configuration: Default::default(),
    };
    let mut bytes = Vec::new();
    write_host_frame(&mut bytes, &message).expect("frame should be written");
    let decoded = read_host_frame(&mut Cursor::new(bytes))
        .expect("frame should be read")
        .expect("frame should exist");
    assert_eq!(decoded, message);
}

#[test]
fn clean_eof_is_not_an_error() {
    assert_eq!(
        read_host_frame(&mut Cursor::new([])).expect("read should succeed"),
        None
    );
}

#[test]
fn truncated_length_is_rejected() {
    let error =
        read_host_frame(&mut Cursor::new([1, 2])).expect_err("truncated length should fail");
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
        write_host_frame(&mut bytes, &message).unwrap();
        assert!(bytes.len() > MAX_EXTENSION_FRAME_BYTES);
        let mut reader = Cursor::new(bytes);
        assert_eq!(read_host_frame(&mut reader).unwrap(), Some(message));
        assert_eq!(read_host_frame(&mut reader).unwrap(), None);
    }
}

#[test]
fn extension_headers_are_bounded_before_the_payload_is_read() {
    let header = ((MAX_EXTENSION_FRAME_BYTES + 1) as u32).to_le_bytes();
    assert!(matches!(
        read_extension_frame(&mut Cursor::new(header)),
        Err(FrameError::InvalidLength(_))
    ));
    let oversized = Message::Error {
        request_id: None,
        code: "error".into(),
        message: "x".repeat(MAX_EXTENSION_FRAME_BYTES),
    };
    let mut bytes = Vec::new();
    assert!(matches!(
        write_extension_frame(&mut bytes, &oversized),
        Err(FrameError::InvalidLength(_))
    ));
    assert!(bytes.is_empty());
}

#[test]
fn truncated_large_host_payload_is_not_a_configuration() {
    let header = ((MAX_EXTENSION_FRAME_BYTES + 1) as u32).to_le_bytes();
    assert!(
        matches!(read_host_frame(&mut Cursor::new(header)), Err(FrameError::Io(error))
        if error.kind() == std::io::ErrorKind::UnexpectedEof)
    );
}

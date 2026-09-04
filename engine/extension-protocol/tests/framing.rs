use std::io::Cursor;

use nanika_protocol::{FrameError, Message, PROTOCOL_NAME, read_frame, write_frame};

#[test]
fn round_trips_a_message() {
    let message = Message::Initialize {
        request_id: "request-1".to_owned(),
        protocol: PROTOCOL_NAME.to_owned(),
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

use std::io;

use futures::TryStreamExt;
use futures::io::Cursor;

use crate::incoming_lines;

#[test]
fn line_reader_accepts_crlf_and_eof_terminated_frames() {
    async_io::block_on(async {
        let mut reader = incoming_lines(Cursor::new(b"first\r\nsecond".to_vec()));

        assert_eq!(
            reader.try_next().await.expect("first line"),
            Some("first".to_owned())
        );
        assert_eq!(
            reader.try_next().await.expect("second line"),
            Some("second".to_owned())
        );
        assert_eq!(reader.try_next().await.expect("end of input"), None);
    });
}

#[test]
fn line_reader_rejects_invalid_utf8_frames() {
    async_io::block_on(async {
        let mut invalid = incoming_lines(Cursor::new(vec![0xff, b'\n']));
        let error = invalid
            .try_next()
            .await
            .expect_err("invalid UTF-8 must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    });
}

#[test]
fn large_line_preserves_the_following_frame() {
    async_io::block_on(async {
        let payload = "x".repeat(9 * 1024 * 1024);
        let mut reader = incoming_lines(Cursor::new(format!("{payload}\r\nnext\n").into_bytes()));
        assert_eq!(reader.try_next().await.unwrap(), Some(payload));
        assert_eq!(reader.try_next().await.unwrap().as_deref(), Some("next"));
        assert_eq!(reader.try_next().await.unwrap(), None);
    });
}

#[test]
fn unterminated_oversized_line_stops_the_stream() {
    use futures::io::{AsyncReadExt, repeat};
    async_io::block_on(async {
        let source = repeat(b'x').take(nanika_protocol::MAX_EXTENSION_MESSAGE_BYTES as u64 + 2);
        let mut reader = incoming_lines(source);
        let error = reader.try_next().await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("exceeds"));
        assert_eq!(reader.try_next().await.unwrap(), None);
    });
}

use std::io;

use futures::io::{BufReader, Cursor};

use crate::read_acp_line;

#[test]
fn line_reader_accepts_crlf_and_eof_terminated_frames() {
    async_io::block_on(async {
        let mut reader = BufReader::new(Cursor::new(b"first\r\nsecond".to_vec()));

        assert_eq!(
            read_acp_line(&mut reader).await.expect("first line"),
            Some("first".to_owned())
        );
        assert_eq!(
            read_acp_line(&mut reader).await.expect("second line"),
            Some("second".to_owned())
        );
        assert_eq!(
            read_acp_line(&mut reader).await.expect("end of input"),
            None
        );
    });
}

#[test]
fn line_reader_rejects_invalid_utf8_frames() {
    async_io::block_on(async {
        let mut invalid = BufReader::new(Cursor::new(vec![0xff, b'\n']));
        let error = read_acp_line(&mut invalid)
            .await
            .expect_err("invalid UTF-8 must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    });
}

#[test]
fn large_line_preserves_the_following_frame() {
    async_io::block_on(async {
        let payload = "x".repeat(9 * 1024 * 1024);
        let mut reader = BufReader::new(Cursor::new(format!("{payload}\r\nnext\n").into_bytes()));
        assert_eq!(read_acp_line(&mut reader).await.unwrap(), Some(payload));
        assert_eq!(
            read_acp_line(&mut reader).await.unwrap().as_deref(),
            Some("next")
        );
        assert_eq!(read_acp_line(&mut reader).await.unwrap(), None);
    });
}

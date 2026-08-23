use std::io;

use futures::io::{BufReader, Cursor};

use crate::read_bounded_line;

#[test]
fn bounded_line_reader_accepts_crlf_and_eof_terminated_frames() {
    async_io::block_on(async {
        let mut reader = BufReader::new(Cursor::new(b"first\r\nsecond".to_vec()));

        assert_eq!(
            read_bounded_line(&mut reader, 16)
                .await
                .expect("first line"),
            Some("first".to_owned())
        );
        assert_eq!(
            read_bounded_line(&mut reader, 16)
                .await
                .expect("second line"),
            Some("second".to_owned())
        );
        assert_eq!(
            read_bounded_line(&mut reader, 16)
                .await
                .expect("end of input"),
            None
        );
    });
}

#[test]
fn bounded_line_reader_rejects_oversized_and_invalid_utf8_frames() {
    async_io::block_on(async {
        let mut oversized = BufReader::new(Cursor::new(b"12345\n".to_vec()));
        let error = read_bounded_line(&mut oversized, 4)
            .await
            .expect_err("oversized frame must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);

        let mut invalid = BufReader::new(Cursor::new(vec![0xff, b'\n']));
        let error = read_bounded_line(&mut invalid, 4)
            .await
            .expect_err("invalid UTF-8 must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    });
}

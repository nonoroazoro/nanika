use std::io::Write;

use crate::BoundedLogWriter;

#[test]
fn writer_drops_bytes_after_the_hard_limit() {
    let mut writer = BoundedLogWriter::new(Vec::new(), 5);

    writer
        .write_all(b"1234")
        .expect("first write should succeed");
    writer
        .write_all(b"56789")
        .expect("overflow should be dropped without blocking the logger");

    assert_eq!(writer.into_inner(), b"12345");
}

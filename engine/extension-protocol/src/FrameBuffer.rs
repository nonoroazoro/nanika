use std::io::{self, Write};

/// Complete serialization before writing a header, without allocating beyond the frame budget.
#[derive(Default)]
pub(crate) struct FrameBuffer(pub(crate) Vec<u8>);

impl Write for FrameBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > crate::MAX_EXTENSION_MESSAGE_BYTES - self.0.len() {
            return Err(io::Error::other("extension message exceeds 64 MiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

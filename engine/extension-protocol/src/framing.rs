use std::io::{Read, Write};

use crate::{FrameError, Message};

/// Write one message with its 32-bit byte length. Neither direction has a byte quota.
pub fn write_frame(writer: &mut impl Write, message: &Message) -> Result<(), FrameError> {
    let payload =
        serde_json::to_vec(message).map_err(|error| FrameError::Json(error.to_string()))?;
    let length =
        u32::try_from(payload.len()).map_err(|_| FrameError::InvalidLength(payload.len()))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// Read one complete message. Clean EOF is distinct from an interrupted frame.
pub fn read_frame(reader: &mut impl Read) -> Result<Option<Message>, FrameError> {
    let mut length_bytes = [0; 4];
    match reader.read_exact(&mut length_bytes[..1]) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(FrameError::Io(error)),
    }
    reader.read_exact(&mut length_bytes[1..])?;
    let length = u32::from_le_bytes(length_bytes);
    let mut payload = Vec::new();
    // A length header is a boundary, not proof that the sender has supplied that much data.
    // Grow with received bytes and keep the next frame outside this reader.
    let mut frame = reader.take(u64::from(length));
    frame.read_to_end(&mut payload)?;
    if frame.limit() != 0 {
        return Err(FrameError::Io(std::io::ErrorKind::UnexpectedEof.into()));
    }
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| FrameError::Json(error.to_string()))
}

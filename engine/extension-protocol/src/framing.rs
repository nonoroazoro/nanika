use std::io::{Read, Write};

use crate::{FrameError, MAX_FRAME_BYTES, Message};

/// Write one little-endian length-delimited JSON frame.
pub fn write_frame(writer: &mut impl Write, message: &Message) -> Result<(), FrameError> {
    let payload =
        serde_json::to_vec(message).map_err(|error| FrameError::Json(error.to_string()))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::InvalidLength(payload.len()));
    }
    let length =
        u32::try_from(payload.len()).map_err(|_| FrameError::InvalidLength(payload.len()))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// Read one frame, returning `None` only when the stream closes before a frame starts.
pub fn read_frame(reader: &mut impl Read) -> Result<Option<Message>, FrameError> {
    let mut length_bytes = [0; 4];
    match reader.read_exact(&mut length_bytes[..1]) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(FrameError::Io(error)),
    }
    reader.read_exact(&mut length_bytes[1..])?;
    let length = u32::from_le_bytes(length_bytes) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(FrameError::InvalidLength(length));
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| FrameError::Json(error.to_string()))
}

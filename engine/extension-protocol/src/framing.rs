use std::io::{Read, Write};

use crate::{FrameError, MAX_EXTENSION_FRAME_BYTES, Message};

/// Write a host-owned frame, including complete configuration snapshots.
pub fn write_host_frame(writer: &mut impl Write, message: &Message) -> Result<(), FrameError> {
    _write_frame(writer, message, None)
}

/// Write an extension response within the host's inbound allocation budget.
pub fn write_extension_frame(writer: &mut impl Write, message: &Message) -> Result<(), FrameError> {
    _write_frame(writer, message, Some(MAX_EXTENSION_FRAME_BYTES))
}

/// Read the owning host's stdin stream, including complete configuration snapshots.
/// The host validates configuration against the extension's schema before transmission.
pub fn read_host_frame(reader: &mut impl Read) -> Result<Option<Message>, FrameError> {
    _read_frame(reader, None)
}

/// Read an untrusted extension's stdout stream. Reject oversized headers before allocation.
pub fn read_extension_frame(reader: &mut impl Read) -> Result<Option<Message>, FrameError> {
    _read_frame(reader, Some(MAX_EXTENSION_FRAME_BYTES))
}

fn _write_frame(
    writer: &mut impl Write,
    message: &Message,
    maximum: Option<usize>,
) -> Result<(), FrameError> {
    let payload =
        serde_json::to_vec(message).map_err(|error| FrameError::Json(error.to_string()))?;
    if maximum.is_some_and(|maximum| payload.len() > maximum) {
        return Err(FrameError::InvalidLength(payload.len()));
    }
    let length =
        u32::try_from(payload.len()).map_err(|_| FrameError::InvalidLength(payload.len()))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

fn _read_frame(
    reader: &mut impl Read,
    maximum: Option<usize>,
) -> Result<Option<Message>, FrameError> {
    let mut length_bytes = [0; 4];
    match reader.read_exact(&mut length_bytes[..1]) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(FrameError::Io(error)),
    }
    reader.read_exact(&mut length_bytes[1..])?;
    let length = u32::from_le_bytes(length_bytes) as usize;
    if maximum.is_some_and(|maximum| length > maximum) {
        return Err(FrameError::InvalidLength(length));
    }
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(length)
        .map_err(std::io::Error::other)?;
    payload.resize(length, 0);
    reader.read_exact(&mut payload)?;
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| FrameError::Json(error.to_string()))
}

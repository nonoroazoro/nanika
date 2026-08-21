//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

/// Current Nanika extension protocol identifier.
pub const PROTOCOL_NAME: &str = "nanika.extension.v1";

/// Maximum encoded protocol frame accepted by the host.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// One request or response on the extension protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    Initialize {
        request_id: String,
        protocol: String,
    },
    Initialized {
        request_id: String,
        protocol: String,
    },
    Query {
        request_id: String,
        generation: u64,
        query: String,
    },
    Snapshot {
        request_id: String,
        generation: u64,
        entries: Vec<Candidate>,
    },
    Invoke {
        request_id: String,
        generation: u64,
        action_id: String,
    },
    Result {
        request_id: String,
        generation: u64,
    },
    Cancel {
        request_id: String,
        generation: u64,
    },
    Shutdown {
        request_id: String,
    },
    ShutdownAck {
        request_id: String,
    },
    Error {
        request_id: Option<String>,
        code: String,
        message: String,
    },
}

/// A bounded searchable result contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub entry_id: String,
    pub title: String,
    pub action_id: String,
}

/// Errors raised while framing or decoding protocol messages.
#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    InvalidLength(usize),
    Json(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::InvalidLength(length) => write!(formatter, "invalid frame length: {length}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

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

/// Read one frame, returning `None` only when the stream is cleanly closed before a frame starts.
pub fn read_frame(reader: &mut impl Read) -> Result<Option<Message>, FrameError> {
    let mut length_bytes = [0; 4];
    match reader.read_exact(&mut length_bytes[..1]) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{Message, PROTOCOL_NAME, read_frame, write_frame};

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
            matches!(error, super::FrameError::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof)
        );
    }
}

//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

/// Current Nanika extension protocol identifier.
pub const PROTOCOL_NAME: &str = "nanika.extension.v1";

/// Maximum encoded protocol frame accepted by the host.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Message categories reserved by the universal extension protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Initialize,
    Query,
    Invoke,
    Cancel,
    Shutdown,
    Initialized,
    Snapshot,
    Result,
    Error,
    ShutdownAck,
}

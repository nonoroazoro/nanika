use nanika_protocol::Message;

use crate::ScanReport;

/// Protocol input and discovery completion serialized onto the runtime loop.
pub enum RuntimeEvent {
    Protocol(Message),
    ProtocolClosed,
    ProtocolError(String),
    ScanFinished {
        request_id: Option<String>,
        response_generation: u64,
        result: Result<ScanReport, String>,
    },
}

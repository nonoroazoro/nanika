use nanika_protocol::Message;

use crate::ScanReport;

/// Protocol input and discovery completion serialized onto the runtime loop.
pub enum RuntimeEvent {
    Protocol(Message),
    CandidatesChanged,
    ProtocolClosed,
    ProtocolError(String),
    ScanProgress {
        request_id: String,
        progress: nanika_protocol::OperationProgress,
    },
    ScanFinished {
        request_id: Option<String>,
        response_generation: u64,
        result: Result<ScanReport, String>,
    },
}

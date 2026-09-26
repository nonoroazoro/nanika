use nanika_protocol::Message;

pub(crate) enum RuntimeEvent {
    Protocol(Message),
    ProtocolClosed,
    ProtocolError(String),
    CatalogUpdated {
        entry_ids: Vec<String>,
    },
    ScanFinished {
        request_id: Option<String>,
        result: Result<(), String>,
    },
}

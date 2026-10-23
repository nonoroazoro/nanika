#[derive(Debug)]
pub(crate) struct ExtensionTextRead {
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
    pub(crate) view_id: String,
    pub(crate) read: nanika_protocol::TextRead,
    pub(crate) completion:
        std::sync::mpsc::Sender<Result<nanika_protocol::TextChunk, crate::OperationError>>
}

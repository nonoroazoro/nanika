/// Terminal result channel for an admitted native host operation.
pub type HostServiceReceipt =
    std::sync::mpsc::Receiver<Result<nanika_protocol::HostServiceResponse, String>>;

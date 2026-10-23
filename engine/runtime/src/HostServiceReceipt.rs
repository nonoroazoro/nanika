/// Terminal native result channel for an admitted host operation.
/// An error is a reported failure; disconnection leaves completion unconfirmed.
pub type HostServiceReceipt =
    std::sync::mpsc::Receiver<Result<nanika_protocol::HostServiceResponse, String>>;

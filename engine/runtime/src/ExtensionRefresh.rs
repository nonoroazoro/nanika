/// One explicit extension refresh generation.
#[derive(Debug)]
pub(crate) struct ExtensionRefresh {
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
    pub(crate) completion: std::sync::mpsc::SyncSender<Result<(), String>>,
}

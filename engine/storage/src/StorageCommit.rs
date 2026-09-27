use std::sync::mpsc::Receiver;

use crate::StorageQueueError;

/// Optional durable acknowledgement of an accepted write. Dropping it neither
/// cancels the write nor suppresses owner diagnostics. It does not await search.
#[derive(Debug)]
pub struct StorageCommit {
    _response: Receiver<Result<(), String>>,
}

impl StorageCommit {
    pub(crate) fn new(response: Receiver<Result<(), String>>) -> Self {
        Self {
            _response: response,
        }
    }

    pub fn wait(self) -> Result<(), StorageQueueError> {
        self._response
            .recv()
            .map_err(|_| StorageQueueError::Closed)?
            .map_err(StorageQueueError::Operation)
    }
}

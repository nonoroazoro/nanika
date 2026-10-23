use crate::ConfigurationSaveOutcome;
use std::sync::mpsc::Receiver;

/// Accepted work belongs to the runtime and continues if the caller drops this receipt.
#[derive(Debug)]
pub struct ConfigurationSaveReceipt(pub(crate) Receiver<ConfigurationSaveOutcome>);

impl ConfigurationSaveReceipt {
    pub fn wait(self) -> Result<ConfigurationSaveOutcome, crate::OperationError> {
        self.0.recv().map_err(|_| {
            crate::OperationError::new(crate::OperationErrorCode::ConfigurationUnconfirmed)
        })
    }
}

#[cfg(test)]
#[path = "../tests/ConfigurationSaveReceipt.rs"]
mod tests;

use crate::ConfigurationSaveOutcome;
use std::sync::mpsc::Receiver;

/// Accepted work belongs to the runtime and continues if the caller drops this receipt.
#[derive(Debug)]
pub struct ConfigurationSaveReceipt(pub(crate) Receiver<ConfigurationSaveOutcome>);

impl ConfigurationSaveReceipt {
    pub fn wait(self) -> Result<ConfigurationSaveOutcome, String> {
        self.0
            .recv()
            .map_err(|_| "Configuration operation ended without a result.".to_owned())
    }
}

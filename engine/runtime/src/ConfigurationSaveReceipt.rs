use std::sync::mpsc::Receiver;

use crate::ConfigurationSaveOutcome;

/// Persistence has completed. Waiting for extension application is optional and separate.
#[derive(Debug)]
pub enum ConfigurationSaveReceipt {
    Complete(ConfigurationSaveOutcome),
    Pending(Receiver<Result<(), String>>),
}

impl ConfigurationSaveReceipt {
    pub fn wait(self) -> ConfigurationSaveOutcome {
        match self {
            Self::Complete(outcome) => outcome,
            Self::Pending(received) => match received.recv() {
                Ok(Ok(())) => ConfigurationSaveOutcome::Applied,
                Ok(Err(error)) => ConfigurationSaveOutcome::ApplyFailed(error),
                Err(_) => ConfigurationSaveOutcome::ApplyFailed(
                    "Extension closed without a configuration result.".to_owned(),
                ),
            },
        }
    }
}

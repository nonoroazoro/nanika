use crate::{RuntimeOutputUpdate, RuntimeSettingsUpdate};

/// Non-blocking batch of updates produced by extension workers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeUpdateBatch {
    pub outputs: Vec<RuntimeOutputUpdate>,
    pub settings: Vec<RuntimeSettingsUpdate>,
}

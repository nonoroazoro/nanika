use crate::{
    RuntimeInvocationUpdate, RuntimeOutputUpdate, RuntimeSettingsUpdate, RuntimeViewUpdate,
};

/// Non-blocking batch of updates produced by extension workers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeUpdateBatch {
    pub invocations: Vec<RuntimeInvocationUpdate>,
    pub outputs: Vec<RuntimeOutputUpdate>,
    pub settings: Vec<RuntimeSettingsUpdate>,
    pub views: Vec<RuntimeViewUpdate>,
}

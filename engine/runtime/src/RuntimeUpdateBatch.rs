use crate::RuntimeOutputUpdate;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeUpdateBatch {
    pub outputs: Vec<RuntimeOutputUpdate>,
}

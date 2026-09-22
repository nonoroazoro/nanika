use crate::{RuntimeConfigurationUpdate, RuntimeOutputUpdate};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeUpdateBatch {
    pub outputs: Vec<RuntimeOutputUpdate>,
    pub configurations: Vec<RuntimeConfigurationUpdate>,
}

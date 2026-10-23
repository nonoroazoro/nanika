use crate::MessageContract;
use serde::Serialize;
use std::collections::BTreeMap;

/// Host-validated source and inferred argument contracts for one immutable frontend bundle.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
pub struct PreparedLocale {
    pub sources: Vec<String>,
    pub contracts: BTreeMap<String, MessageContract>
}

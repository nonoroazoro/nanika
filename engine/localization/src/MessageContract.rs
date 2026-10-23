use serde::Serialize;
use std::collections::BTreeSet;

/// Inferred from FTL, including references and every selector branch.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MessageContract {
    pub arguments: BTreeSet<String>,
    #[serde(skip)]
    pub(crate) expanded_bytes: usize,
    #[serde(skip)]
    pub(crate) depth: usize,
    #[serde(skip)]
    pub(crate) placeables: usize
}

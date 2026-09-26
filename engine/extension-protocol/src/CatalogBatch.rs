use crate::Candidate;
use serde::{Deserialize, Serialize};

/// Ordered parts of one atomic, query-independent catalog publication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogBatch {
    pub transaction: u64,
    pub index: u64,
    pub replace: bool,
    pub complete: bool,
    pub entries: Vec<Candidate>,
    pub removed: Vec<String>,
}

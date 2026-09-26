use crate::Candidate;
use std::collections::VecDeque;
use std::sync::Arc;

pub(crate) struct CatalogPublication {
    pub(crate) _transaction: u64,
    pub(crate) _index: u64,
    pub(crate) _replace: bool,
    pub(crate) _complete: bool,
    pub(crate) _entries: VecDeque<Arc<Candidate>>,
    pub(crate) _removed: VecDeque<String>,
}

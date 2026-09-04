use serde::Serialize;

use crate::SearchResult;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RootSearchSnapshot {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) results: Vec<SearchResult>,
    pub(crate) complete: bool,
}

impl RootSearchSnapshot {
    pub(crate) fn pending(generation: u64, query: String) -> Self {
        Self {
            generation,
            query,
            results: Vec::new(),
            complete: false,
        }
    }

    pub(crate) fn from_engine(snapshot: &nanika_search::SearchSnapshot, query: String) -> Self {
        Self {
            generation: snapshot.generation,
            query,
            results: snapshot
                .results
                .iter()
                .map(|ranked| SearchResult::from_candidate(&ranked.candidate))
                .collect(),
            complete: true,
        }
    }
}

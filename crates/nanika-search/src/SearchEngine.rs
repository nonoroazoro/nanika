use crate::{Candidate, MatchContext, SearchSnapshot, UsageMap, ranking};

/// Query owner that reuses matcher scratch memory across generations.
pub struct SearchEngine {
    generation: u64,
    context: MatchContext,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            generation: 0,
            context: MatchContext::new(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn query(
        &mut self,
        query: &str,
        candidates: &[Candidate],
        usage: &UsageMap,
        now: u64,
    ) -> SearchSnapshot {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.rank(self.generation, query, candidates.iter(), usage, now)
    }

    pub(crate) fn rank<'a>(
        &mut self,
        generation: u64,
        query: &str,
        candidates: impl Iterator<Item = &'a Candidate>,
        usage: &UsageMap,
        now: u64,
    ) -> SearchSnapshot {
        ranking::rank(generation, query, candidates, usage, now, &mut self.context)
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Rank a standalone candidate slice. Long-lived owners should reuse `SearchEngine`.
pub fn rank_candidates(
    generation: u64,
    query: &str,
    candidates: &[Candidate],
    usage: &UsageMap,
    now: u64,
) -> SearchSnapshot {
    SearchEngine::new().rank(generation, query, candidates.iter(), usage, now)
}

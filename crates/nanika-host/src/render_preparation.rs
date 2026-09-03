use nanika_search::{RankedCandidate, SearchSnapshot};

pub const MAX_ROOT_RESULTS: usize = 100;
pub const OVERLAY_WIDTH_POINTS: f32 = 720.0;
pub const OVERLAY_HEIGHT_POINTS: f32 = 480.0;

/// Select the bounded result slice rendered by the overlay without allocation.
pub fn prepare_visible_results(
    snapshot: &SearchSnapshot,
    generation: u64,
    selected_index: usize,
) -> impl Iterator<Item = (usize, &RankedCandidate, bool)> {
    let results = if snapshot.generation == generation {
        snapshot.results.as_slice()
    } else {
        &[]
    };
    results
        .iter()
        .take(MAX_ROOT_RESULTS)
        .enumerate()
        .map(move |(index, result)| (index, result, index == selected_index))
}

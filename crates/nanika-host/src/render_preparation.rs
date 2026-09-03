use nanika_search::{RankedCandidate, SearchSnapshot};

use crate::DesignSystem;

pub const MAX_ROOT_RESULTS: usize = 100;
pub const OVERLAY_WIDTH_POINTS: f32 = 720.0;
pub const OVERLAY_HEIGHT_POINTS: f32 = 480.0;

pub(crate) fn root_overlay_height_points(result_count: usize) -> f32 {
    let visible_rows = result_count.clamp(1, DesignSystem::ROOT_VISIBLE_ROWS);
    DesignSystem::ROOT_CHROME_HEIGHT + visible_rows as f32 * DesignSystem::ROOT_RESULT_ROW_HEIGHT
}

pub(crate) fn root_selection_scroll_target(
    selected_index: usize,
    current_offset: f32,
    viewport_height: f32,
) -> Option<f32> {
    const OFFSET_TOLERANCE: f32 = 0.5;
    let row_top = selected_index as f32 * DesignSystem::ROOT_RESULT_ROW_HEIGHT;
    let row_bottom = row_top + DesignSystem::ROOT_RESULT_ROW_HEIGHT;
    if row_top + OFFSET_TOLERANCE < current_offset {
        Some(row_top)
    } else if row_bottom > current_offset + viewport_height + OFFSET_TOLERANCE {
        Some((row_bottom - viewport_height).max(0.0))
    } else {
        None
    }
}

pub(crate) fn row_crosses_viewport_edge(row: egui::Rect, viewport: egui::Rect) -> bool {
    const EDGE_TOLERANCE: f32 = 0.5;
    row.top() + EDGE_TOLERANCE < viewport.top() || row.bottom() > viewport.bottom() + EDGE_TOLERANCE
}

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

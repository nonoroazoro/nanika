use nanika_search::{Candidate, RankedCandidate, SearchSnapshot};

use crate::{
    prepare_visible_results, root_overlay_height_points, root_selection_scroll_target,
    row_crosses_viewport_edge,
};

#[test]
fn preparation_supports_the_complete_root_result_limit_and_tracks_selection() {
    let snapshot = snapshot(7, 10);
    let prepared = prepare_visible_results(&snapshot, 7, 3).collect::<Vec<_>>();

    assert_eq!(prepared.len(), 10);
    assert!(prepared[3].2);
    assert_eq!(prepared[9].1.candidate.entry_id(), "entry-9");
}

#[test]
fn preparation_rejects_stale_generation() {
    let snapshot = snapshot(6, 10);

    assert_eq!(prepare_visible_results(&snapshot, 7, 0).count(), 0);
}

#[test]
fn root_height_tracks_one_to_eight_complete_rows() {
    assert_eq!(root_overlay_height_points(0), 141.0);
    assert_eq!(root_overlay_height_points(1), 141.0);
    assert_eq!(root_overlay_height_points(3), 237.0);
    assert_eq!(root_overlay_height_points(8), 477.0);
    assert_eq!(root_overlay_height_points(100), 477.0);
}

#[test]
fn root_selection_scrolls_only_after_crossing_a_viewport_edge() {
    let viewport_height = 4.0 * crate::DesignSystem::ROOT_RESULT_ROW_HEIGHT;

    assert_eq!(root_selection_scroll_target(0, 0.0, viewport_height), None);
    assert_eq!(root_selection_scroll_target(3, 0.0, viewport_height), None);
    assert_eq!(
        root_selection_scroll_target(4, 0.0, viewport_height),
        Some(crate::DesignSystem::ROOT_RESULT_ROW_HEIGHT)
    );
    assert_eq!(
        root_selection_scroll_target(1, 96.0, viewport_height),
        Some(crate::DesignSystem::ROOT_RESULT_ROW_HEIGHT)
    );
    assert_eq!(root_selection_scroll_target(2, 96.0, viewport_height), None);
}

#[test]
fn moving_up_after_the_first_downward_scroll_keeps_the_viewport_stable() {
    let row_height = crate::DesignSystem::ROOT_RESULT_ROW_HEIGHT;
    let viewport_height = 4.0 * row_height;
    let offset_after_first_scroll = row_height;

    for selected_index in [3, 2, 1] {
        assert_eq!(
            root_selection_scroll_target(
                selected_index,
                offset_after_first_scroll,
                viewport_height,
            ),
            None
        );
    }
    assert_eq!(
        root_selection_scroll_target(0, offset_after_first_scroll, viewport_height),
        Some(0.0)
    );
}

#[test]
fn extension_row_scrolls_only_outside_the_visible_viewport() {
    let viewport = egui::Rect::from_min_max(egui::pos2(0.0, 100.0), egui::pos2(320.0, 300.0));

    assert!(!row_crosses_viewport_edge(
        egui::Rect::from_min_max(egui::pos2(0.0, 100.0), egui::pos2(320.0, 148.0)),
        viewport,
    ));
    assert!(!row_crosses_viewport_edge(
        egui::Rect::from_min_max(egui::pos2(0.0, 252.0), egui::pos2(320.0, 300.0)),
        viewport,
    ));
    assert!(row_crosses_viewport_edge(
        egui::Rect::from_min_max(egui::pos2(0.0, 51.0), egui::pos2(320.0, 99.0)),
        viewport,
    ));
    assert!(row_crosses_viewport_edge(
        egui::Rect::from_min_max(egui::pos2(0.0, 301.0), egui::pos2(320.0, 349.0)),
        viewport,
    ));
}

fn snapshot(generation: u64, count: usize) -> SearchSnapshot {
    SearchSnapshot {
        generation,
        normalized_query: "application".to_owned(),
        results: (0..count)
            .map(|index| RankedCandidate {
                candidate: Candidate::new(
                    "benchmark",
                    format!("entry-{index}"),
                    format!("Application {index}"),
                    "launch",
                    Vec::new(),
                ),
                lexical_tier: 1,
                fuzzy_score: 100,
                contextual_boost: 0,
            })
            .collect(),
    }
}

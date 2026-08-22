use crate::host_app::{maximum_visible_result_index, truncate_chars};

#[test]
fn selection_stays_within_rendered_results() {
    assert_eq!(maximum_visible_result_index(0), 0);
    assert_eq!(maximum_visible_result_index(3), 2);
    assert_eq!(maximum_visible_result_index(100), 7);
}

#[test]
fn query_truncation_preserves_utf8() {
    let mut query = "一二三four".to_owned();
    truncate_chars(&mut query, 4);
    assert_eq!(query, "一二三f");
}

use crate::host_app::{
    extension_startup_user_message, maximum_visible_result_index, next_list_item,
    previous_list_item, selected_execution, should_render_runtime_error, truncate_chars,
};
use crate::{DiagnosticCode, ExtensionStartupError, HostDiagnostic};
use nanika_protocol::ListItem;
use nanika_search::{Candidate, RankedCandidate, SearchSnapshot};

#[test]
fn selection_stays_within_rendered_results() {
    assert_eq!(maximum_visible_result_index(0), 0);
    assert_eq!(maximum_visible_result_index(3), 2);
    assert_eq!(maximum_visible_result_index(100), 99);
}

#[test]
fn list_navigation_stops_at_both_boundaries() {
    let first = list_item("first");
    let second = list_item("second");
    let items = vec![&first, &second];

    assert!(previous_list_item(&items, Some(0)).is_none());
    assert_eq!(
        previous_list_item(&items, Some(1)).map(|item| &item.id),
        Some(&first.id)
    );
    assert_eq!(
        next_list_item(&items, None).map(|item| &item.id),
        Some(&first.id)
    );
    assert_eq!(
        next_list_item(&items, Some(0)).map(|item| &item.id),
        Some(&second.id)
    );
    assert!(next_list_item(&items, Some(1)).is_none());
}

#[test]
fn query_truncation_preserves_utf8() {
    let mut query = "一二三four".to_owned();
    truncate_chars(&mut query, 4);
    assert_eq!(query, "一二三f");
}

#[test]
fn selected_item_execution_does_not_require_query_text() {
    let snapshot = SearchSnapshot {
        generation: 7,
        normalized_query: String::new(),
        results: vec![RankedCandidate {
            candidate: Candidate::new(
                "com.nanika.application",
                "app.example",
                "Example",
                "run",
                Vec::new(),
            ),
            lexical_tier: 0,
            fuzzy_score: 0,
            contextual_boost: 0,
        }],
    };

    let (candidate, query_context) =
        selected_execution(Some(&snapshot), snapshot.generation, 0, "   ")
            .expect("selected item should be executable");

    assert_eq!(candidate.entry_id(), "app.example");
    assert!(query_context.is_empty());
}

#[test]
fn repeated_runtime_messages_render_once() {
    let errors = vec![
        HostDiagnostic::new(
            DiagnosticCode::ExtensionUnavailable,
            "start extension",
            "Some Nanika features are unavailable.",
        ),
        HostDiagnostic::new(
            DiagnosticCode::ExtensionUnavailable,
            "start extension",
            "Some Nanika features are unavailable.",
        ),
        HostDiagnostic::new(
            DiagnosticCode::StorageUnavailable,
            "start storage",
            "History is unavailable.",
        ),
    ];

    assert!(should_render_runtime_error(&errors, 0));
    assert!(!should_render_runtime_error(&errors, 1));
    assert!(should_render_runtime_error(&errors, 2));
    assert!(!should_render_runtime_error(&errors, 3));
}

#[test]
fn extension_startup_message_names_user_features() {
    let errors = vec![
        ExtensionStartupError::new("com.nanika.application", "missing binary"),
        ExtensionStartupError::new("com.nanika.calculator", "missing binary"),
    ];

    assert_eq!(
        extension_startup_user_message(&errors),
        "App search and calculator are unavailable. Restart Nanika. If the problem continues, reinstall Nanika or the affected add-on."
    );
}

fn list_item(id: &str) -> ListItem {
    ListItem {
        id: id.to_owned(),
        title: id.to_owned(),
        subtitle: None,
        actions: Vec::new(),
    }
}

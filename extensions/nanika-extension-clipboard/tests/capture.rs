use crate::{files_within_limits, text_within_limits};

#[test]
fn oversized_clipboard_content_is_rejected_instead_of_truncated() {
    assert!(!text_within_limits(&"x".repeat(1024 * 1024 + 1)));
    assert!(!files_within_limits(&vec!["file".to_owned(); 257], 1024));
    assert!(!files_within_limits(&["file".to_owned()], 1024 * 1024 + 1));
}

use crate::{
    ExtensionInvocationOutput, InvocationPresentation, MAX_VISIBLE_INVOCATION_OUTPUT_BYTES,
};

#[test]
fn output_deltas_append_only_to_their_invocation() {
    let mut presentation = InvocationPresentation::from_output(output(2, "Hello"));

    assert!(!presentation.append(output(2, " World")));
    assert_eq!(presentation.text, "Hello World");
    assert!(!presentation.append(output(1, "stale")));
    assert_eq!(presentation.text, "Hello World");
    assert!(presentation.append(output(3, "new")));
    assert_eq!(presentation.invocation_id, 3);
    assert_eq!(presentation.text, "new");
}

#[test]
fn visible_output_is_a_bounded_utf8_tail() {
    let prefix = "x".repeat(MAX_VISIBLE_INVOCATION_OUTPUT_BYTES);
    let presentation = InvocationPresentation::from_output(output(1, &(prefix + "世界")));

    let (visible, truncated) = presentation.visible_text();

    assert!(truncated);
    assert!(visible.len() <= MAX_VISIBLE_INVOCATION_OUTPUT_BYTES);
    assert!(visible.ends_with("世界"));
    assert!(visible.is_char_boundary(0));
}

fn output(invocation_id: u64, text: &str) -> ExtensionInvocationOutput {
    ExtensionInvocationOutput {
        invocation_id,
        extension_id: "com.example.agent".to_owned(),
        generation: 1,
        text: text.to_owned(),
    }
}

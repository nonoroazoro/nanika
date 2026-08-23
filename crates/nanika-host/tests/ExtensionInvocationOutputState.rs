use crate::ExtensionInvocationOutputState;

#[test]
fn output_chunks_are_coalesced_until_the_ui_observes_them() {
    let mut state = ExtensionInvocationOutputState::default();

    assert!(state.take_changed().is_none());
    assert!(state.append(11, "com.example.agent", 7, "Hello"));
    assert!(!state.append(11, "com.example.agent", 7, " World"));

    let first = state
        .take_changed()
        .expect("first output batch")
        .pop()
        .expect("first output delta");
    assert_eq!(first.invocation_id, 11);
    assert_eq!(first.extension_id, "com.example.agent");
    assert_eq!(first.generation, 7);
    assert_eq!(first.text, "Hello World");
    assert!(state.take_changed().is_none());

    assert!(state.append(11, "com.example.agent", 7, "!"));
    assert_eq!(
        state.take_changed().expect("updated output batch")[0].text,
        "!"
    );
}

#[test]
fn distinct_invocations_are_not_coalesced() {
    let mut state = ExtensionInvocationOutputState::default();
    state.append(41, "com.example.agent", 1, "first");
    state.append(42, "com.example.agent", 1, "second");

    let outputs = state.take_changed().expect("output batch");
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].invocation_id, 41);
    assert_eq!(outputs[0].text, "first");
    assert_eq!(outputs[1].invocation_id, 42);
    assert_eq!(outputs[1].text, "second");
}

#[test]
fn pending_output_batches_remain_bounded() {
    let mut state = ExtensionInvocationOutputState::default();
    for invocation_id in 1..=17 {
        state.append(invocation_id, "com.example.agent", 1, "output");
    }

    let outputs = state.take_changed().expect("output batch");
    assert_eq!(outputs.len(), 16);
    assert_eq!(outputs[0].invocation_id, 2);
    assert_eq!(outputs[15].invocation_id, 17);
}

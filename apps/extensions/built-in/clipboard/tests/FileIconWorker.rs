use super::*;

#[test]
fn scheduling_is_ordered_deduplicated_and_bounded_at_capacity() {
    let worker = FileIconWorker {
        state: Arc::new((Mutex::new(State::default()), Condvar::new())),
        thread: None,
    };
    assert_eq!(
        worker.schedule([PathBuf::from("selected"), PathBuf::from("selected")]),
        1
    );
    let state = worker.state.0.lock().unwrap();
    assert_eq!(
        state.pending.iter().collect::<Vec<_>>(),
        [&PathBuf::from("selected")]
    );
    drop(state);

    assert_eq!(
        worker.schedule((1..MAX_PENDING_PATHS).map(|index| PathBuf::from(index.to_string()))),
        MAX_PENDING_PATHS - 1
    );
    let before = worker.state.0.lock().unwrap().pending.len();
    assert_eq!(worker.schedule([PathBuf::from("overflow")]), 0);
    assert_eq!(worker.state.0.lock().unwrap().pending.len(), before);
}

#[test]
fn published_resolutions_are_bounded_in_insertion_order() {
    let mut state = State::default();
    for index in 0..=MAX_PENDING_PATHS {
        publish_resolution(
            &mut state,
            PathBuf::from(index.to_string()),
            Some(nanika_protocol::IconReference::new(format!("{index:064x}")).unwrap()),
        );
    }
    assert_eq!(state.resolved.len(), MAX_PENDING_PATHS);
    assert!(!state.resolved.contains_key(&PathBuf::from("0")));
    assert!(
        state
            .resolved
            .contains_key(&PathBuf::from(MAX_PENDING_PATHS.to_string()))
    );
}

#[test]
fn failed_resolution_is_retained_without_exposing_an_icon() {
    let path = PathBuf::from("missing");
    let mut state = State::default();

    publish_resolution(&mut state, path.clone(), None);

    assert!(state.resolved.contains_key(&path));
    assert_eq!(state.resolved[&path], None);
}

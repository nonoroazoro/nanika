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
        _publish_resolution(
            &mut state,
            PathBuf::from(index.to_string()),
            crate::FileIconResolution {
                source: Ok(nanika_protocol::IconReference::new(format!("{index:064x}")).unwrap()),
                icon: Some(nanika_protocol::IconReference::new(format!("{index:064x}")).unwrap()),
            },
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

    _publish_resolution(
        &mut state,
        path.clone(),
        crate::FileIconResolution {
            source: Err(std::io::ErrorKind::NotFound),
            icon: None,
        },
    );

    assert!(state.resolved.contains_key(&path));
    assert_eq!(state.resolved[&path].icon, None);
}

#[test]
fn refresh_revalidates_sources_without_retrying_unchanged_failures() {
    use std::{sync::mpsc, time::Duration};
    let root = std::env::temp_dir().join(format!("nanika-icon-source-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("source.png");
    let icons = root.join("icons");
    let (changed, received) = mpsc::channel();
    let worker =
        FileIconWorker::spawn(icons.clone(), Arc::new(move || changed.send(()).unwrap())).unwrap();
    worker.schedule([path.clone()]);
    received.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(worker.resolution(&path), Some(None));
    worker.schedule([path.clone()]);
    _wait_idle(&worker);
    assert!(
        received.try_recv().is_err(),
        "same missing source must not retry or invalidate"
    );

    let cache = nanika_platform::FileIconCache::new(icons.clone());
    let mut previous = None;
    for content in [b"first".as_slice(), b"replacement".as_slice()] {
        std::fs::write(&path, content).unwrap();
        let reference = cache.reference(&path).unwrap();
        assert_ne!(Some(&reference), previous.as_ref());
        let directory = icons.join(reference.key());
        std::fs::create_dir_all(&directory).unwrap();
        // The worker tests metadata scheduling; platform tests own native extraction.
        for size in [128, 512] {
            std::fs::write(
                directory.join(format!("{size}.png")),
                include_bytes!("../assets/icon.png"),
            )
            .unwrap();
        }
        worker.schedule([path.clone()]);
        received.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(worker.resolution(&path), Some(Some(reference.clone())));
        worker.schedule([path.clone()]);
        _wait_idle(&worker);
        assert!(
            received.try_recv().is_err(),
            "unchanged metadata must not invalidate again"
        );
        previous = Some(reference);
    }
    std::fs::remove_file(&path).unwrap();
    worker.schedule([path.clone()]);
    received.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(worker.resolution(&path), Some(None));
    worker.shutdown().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn _wait_idle(worker: &FileIconWorker) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !worker.state.0.lock().unwrap().known.is_empty() {
        assert!(
            std::time::Instant::now() < deadline,
            "worker did not settle"
        );
        std::thread::yield_now();
    }
}

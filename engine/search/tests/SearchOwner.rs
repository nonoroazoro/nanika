use std::time::{Duration, Instant};

use nanika_search::{
    Candidate, CandidateKind, MAX_QUERY_CHARS, SearchOwner, SearchQueueError, UsageMap,
};

#[test]
fn owner_drops_stale_extension_snapshots() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let handle = owner.handle();
    let stale = handle.begin_query("old").expect("query should enqueue");
    let current = handle.begin_query("tool").expect("query should enqueue");
    handle
        .publish_extension_snapshot(
            "test.extension",
            stale,
            vec![Candidate::new(
                CandidateKind::Action,
                "test.extension",
                "stale",
                "Old",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
        )
        .expect("stale snapshot should enqueue");
    handle
        .publish_extension_snapshot(
            "test.extension",
            current,
            vec![Candidate::new(
                CandidateKind::Action,
                "test.extension",
                "current",
                "Tool",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
        )
        .expect("current snapshot should enqueue");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = handle.latest_snapshot()
            && snapshot.generation == current
            && snapshot.results.len() == 1
        {
            assert_eq!(snapshot.results[0].candidate.entry_id(), "current");
            break;
        }
        assert!(Instant::now() < deadline, "current snapshot should arrive");
        std::thread::yield_now();
    }
    owner.shutdown();
}

#[test]
fn static_catalog_survives_queries_and_preserves_the_dynamic_completion_barrier() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let (sent, received) = std::sync::mpsc::channel();
    handle.set_notifier(std::sync::Arc::new(move || {
        let _ = sent.send(());
    }));
    handle
        .register_static_catalog(
            "static.extension",
            vec![Candidate::new(
                CandidateKind::Action,
                "untrusted.owner",
                "static",
                "Static tool",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
        )
        .unwrap();
    let generation = handle
        .begin_query_with_expected_extensions(
            "tool",
            [
                "static.extension".to_owned(),
                "dynamic.extension".to_owned(),
            ],
        )
        .unwrap();
    assert!(received.recv_timeout(Duration::from_millis(20)).is_err());
    handle
        .publish_extension_snapshot("dynamic.extension", generation, Vec::new())
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let snapshot = handle.latest_snapshot().unwrap();
    assert_eq!(snapshot.generation, generation);
    assert_eq!(snapshot.results.len(), 1);
    assert_eq!(
        snapshot.results[0].candidate.extension_id(),
        "static.extension"
    );
    let next = handle
        .begin_query_with_expected_extensions("static", ["static.extension".to_owned()])
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let snapshot = handle.latest_snapshot().unwrap();
    assert_eq!(snapshot.generation, next);
    assert_eq!(snapshot.results.len(), 1);
    owner.shutdown();
}

#[test]
fn owner_rejects_oversized_queries_before_enqueueing() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let error = owner
        .handle()
        .begin_query("x".repeat(MAX_QUERY_CHARS + 1))
        .expect_err("oversized query should fail");
    assert_eq!(error, SearchQueueError::QueryTooLong);
    owner.shutdown();
}

#[test]
fn owner_coalesces_query_bursts_without_dropping_the_latest_query() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let handle = owner.handle();
    let mut latest_generation = 0;
    for index in 0..1_000 {
        latest_generation = handle
            .begin_query(format!("query {index}"))
            .expect("latest query should never be dropped");
    }

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = handle.latest_snapshot()
            && snapshot.generation == latest_generation
            && snapshot.normalized_query == "query 999"
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "latest query should be published"
        );
        std::thread::yield_now();
    }
    owner.shutdown();
}

#[test]
fn owner_publishes_initial_extension_results_as_one_snapshot() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let handle = owner.handle();
    let generation = handle
        .begin_query_with_expected_extensions(
            "tool",
            ["first.extension".to_owned(), "second.extension".to_owned()],
        )
        .expect("query should enqueue");
    handle
        .publish_extension_snapshot(
            "first.extension",
            generation,
            vec![Candidate::new(
                CandidateKind::Action,
                "first.extension",
                "first",
                "First Tool",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
        )
        .expect("first snapshot should enqueue");

    std::thread::sleep(Duration::from_millis(20));
    assert!(
        handle
            .latest_snapshot()
            .is_none_or(|snapshot| snapshot.generation != generation),
        "partial initial results must not be published"
    );

    handle
        .publish_extension_snapshot(
            "second.extension",
            generation,
            vec![Candidate::new(
                CandidateKind::Action,
                "second.extension",
                "second",
                "Second Tool",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
        )
        .expect("second snapshot should enqueue");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = handle.latest_snapshot()
            && snapshot.generation == generation
            && snapshot.results.len() == 2
        {
            break;
        }
        assert!(Instant::now() < deadline, "combined snapshot should arrive");
        std::thread::yield_now();
    }
    owner.shutdown();
}

#[test]
fn owner_waits_for_the_expected_extension_identities() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let handle = owner.handle();
    let generation = handle
        .begin_query_with_expected_extensions(
            "tool",
            ["first.extension".to_owned(), "second.extension".to_owned()],
        )
        .expect("query should enqueue");
    handle
        .publish_extension_snapshot("unexpected.extension", generation, Vec::new())
        .expect("unexpected snapshot should enqueue");
    handle
        .publish_extension_snapshot("first.extension", generation, Vec::new())
        .expect("first snapshot should enqueue");

    std::thread::sleep(Duration::from_millis(20));
    assert!(
        handle
            .latest_snapshot()
            .is_none_or(|snapshot| snapshot.generation != generation),
        "unexpected workers must not satisfy the initial result barrier"
    );

    handle
        .publish_extension_snapshot("second.extension", generation, Vec::new())
        .expect("second snapshot should enqueue");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if handle
            .latest_snapshot()
            .is_some_and(|snapshot| snapshot.generation == generation)
        {
            break;
        }
        assert!(Instant::now() < deadline, "combined snapshot should arrive");
        std::thread::yield_now();
    }
    owner.shutdown();
}

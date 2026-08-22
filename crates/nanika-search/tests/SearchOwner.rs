use std::time::{Duration, Instant};

use nanika_search::{Candidate, MAX_QUERY_CHARS, SearchOwner, SearchQueueError, UsageMap};

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
                "test.extension",
                "stale",
                "Old",
                "open",
                Vec::new(),
            )],
        )
        .expect("stale snapshot should enqueue");
    handle
        .publish_extension_snapshot(
            "test.extension",
            current,
            vec![Candidate::new(
                "test.extension",
                "current",
                "Tool",
                "open",
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

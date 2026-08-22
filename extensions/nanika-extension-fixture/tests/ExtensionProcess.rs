use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use nanika_host::{
    ExtensionLimits, ExtensionProcess, ExtensionSearchCoordinator, SupervisorError,
    publish_extension_snapshot,
};
use nanika_search::{SearchOwner, UsageMap};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nanika-extension-fixture"))
}

#[test]
fn fixture_completes_handshake_and_shutdown() {
    let fixture = fixture_path();
    assert!(
        fixture.is_file(),
        "fixture is not built: {}",
        fixture.display()
    );

    let mut extension = ExtensionProcess::spawn(fixture).expect("fixture should spawn");
    extension
        .initialize("initialize-1")
        .expect("fixture should initialize");
    extension
        .shutdown("shutdown-1")
        .expect("fixture should shut down");
}

#[test]
fn protocol_operations_require_initialization() {
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    let error = extension
        .query(
            "query-before-initialize",
            1,
            "fixture",
            Duration::from_secs(1),
        )
        .expect_err("query should require initialization");
    assert!(matches!(error, SupervisorError::UnexpectedMessage(_)));
}

#[test]
fn fixture_contributes_a_generation_tagged_search_snapshot() {
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    extension
        .initialize("initialize-query")
        .expect("fixture should initialize");
    let entries = extension
        .query("query-1", 7, "calculator", Duration::from_secs(1))
        .expect("fixture should return candidates");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "calculator");
    assert_eq!(entries[0].aliases, ["fixture alias"]);
    extension
        .shutdown("shutdown-query")
        .expect("fixture should shut down");
}

#[test]
fn fixture_completes_a_generation_tagged_invocation() {
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    extension
        .initialize("initialize-invoke")
        .expect("fixture should initialize");
    extension
        .invoke("invoke-1", 7, "fixture.entry", "fixture.run")
        .expect("fixture action should complete");
    extension
        .shutdown("shutdown-invoke")
        .expect("fixture should shut down");
}

#[test]
fn extension_snapshot_reaches_the_shared_search_owner() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let generation = search
        .begin_query("calculator")
        .expect("query should enqueue");
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    extension
        .initialize("initialize-search-owner")
        .expect("fixture should initialize");
    let entries = extension
        .query(
            "query-search-owner",
            generation,
            "calculator",
            Duration::from_secs(1),
        )
        .expect("fixture should return candidates");
    publish_extension_snapshot(&search, "fixture.extension", generation, entries)
        .expect("snapshot should enqueue");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.generation == generation
            && snapshot.results.len() == 1
        {
            assert_eq!(
                snapshot.results[0].candidate.extension_id(),
                "fixture.extension"
            );
            break;
        }
        assert!(Instant::now() < deadline, "search snapshot should arrive");
        std::thread::yield_now();
    }
    extension
        .shutdown("shutdown-search-owner")
        .expect("fixture should shut down");
    owner.shutdown();
}

#[test]
fn extension_search_worker_dispatches_off_the_caller_thread() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register("fixture.extension", extension, search.clone())
        .expect("worker should register");
    let generation = search
        .begin_query("calculator")
        .expect("query should enqueue");
    coordinator.query(generation, "calculator");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.generation == generation
            && snapshot.results.len() == 1
        {
            assert_eq!(snapshot.results[0].candidate.title(), "calculator");
            break;
        }
        assert!(Instant::now() < deadline, "worker snapshot should arrive");
        std::thread::yield_now();
    }
    drop(coordinator);
    owner.shutdown();
}

#[test]
fn late_snapshot_after_timeout_does_not_poison_the_next_query() {
    let mut extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--delay-first-query".into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    extension
        .initialize("initialize-delayed-query")
        .expect("fixture should initialize");
    let error = extension
        .query("slow-query", 1, "first", Duration::from_millis(10))
        .expect_err("first query should time out");
    assert!(
        matches!(error, SupervisorError::Timeout("query snapshot")),
        "unexpected error: {error:?}"
    );
    let entries = extension
        .query("next-query", 2, "second", Duration::from_secs(1))
        .expect("next query should ignore the late snapshot");
    assert_eq!(entries[0].title, "second");
    extension
        .shutdown("shutdown-after-delayed-query")
        .expect("fixture should shut down");
}

#[test]
fn extension_worker_publishes_incremental_snapshots() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let notifications = Arc::new(AtomicUsize::new(0));
    let notifier_count = Arc::clone(&notifications);
    search.set_notifier(Arc::new(move || {
        notifier_count.fetch_add(1, Ordering::Relaxed);
    }));
    let mut extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--incremental-query".into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    extension
        .initialize("initialize-incremental-query")
        .expect("fixture should initialize");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register("fixture.extension", extension, search.clone())
        .expect("worker should register");
    let generation = search.begin_query("final").expect("query should enqueue");
    coordinator.query(generation, "final");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.generation == generation
            && snapshot
                .results
                .first()
                .is_some_and(|result| result.candidate.title() == "final")
            && notifications.load(Ordering::Relaxed) >= 2
        {
            break;
        }
        assert!(Instant::now() < deadline, "both snapshots should publish");
        std::thread::yield_now();
    }
    drop(coordinator);
    owner.shutdown();
}

#[test]
fn extension_worker_recovers_a_crashed_query_automatically() {
    let marker =
        std::env::temp_dir().join(format!("nanika-extension-recovery-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let mut extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--crash-query-once={}", marker.display()).into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    extension
        .initialize("initialize-crash-recovery")
        .expect("fixture should initialize");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register("fixture.extension", extension, search.clone())
        .expect("worker should register");
    let generation = search
        .begin_query("recovered")
        .expect("query should enqueue");
    coordinator.query(generation, "recovered");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.generation == generation
            && snapshot
                .results
                .first()
                .is_some_and(|result| result.candidate.title() == "recovered")
        {
            break;
        }
        assert!(Instant::now() < deadline, "worker should recover and retry");
        std::thread::yield_now();
    }
    assert!(coordinator.first_error().is_none());
    drop(coordinator);
    owner.shutdown();
    let _ = std::fs::remove_file(marker);
}

#[test]
fn extension_worker_recovers_a_timed_out_query_automatically() {
    let marker =
        std::env::temp_dir().join(format!("nanika-extension-timeout-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let mut extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--hang-query-once={}", marker.display()).into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    extension
        .initialize("initialize-timeout-recovery")
        .expect("fixture should initialize");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register("fixture.extension", extension, search.clone())
        .expect("worker should register");
    let generation = search
        .begin_query("recovered after timeout")
        .expect("query should enqueue");
    coordinator.query(generation, "recovered after timeout");

    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.generation == generation
            && snapshot
                .results
                .first()
                .is_some_and(|result| result.candidate.title() == "recovered after timeout")
        {
            break;
        }
        assert!(Instant::now() < deadline, "worker should recover and retry");
        std::thread::yield_now();
    }
    assert!(coordinator.first_error().is_none());
    drop(coordinator);
    owner.shutdown();
    let _ = std::fs::remove_file(marker);
}

#[test]
fn coordinator_shutdown_cancels_a_running_action() {
    let marker = std::env::temp_dir().join(format!(
        "nanika-extension-action-hang-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&marker);
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let mut extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--hang-invoke={}", marker.display()).into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    extension
        .initialize("initialize-action-cancellation")
        .expect("fixture should initialize");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register("fixture.extension", extension, search)
        .expect("worker should register");
    coordinator
        .invoke(
            "fixture.extension",
            1,
            "fixture.entry",
            "fixture.run",
            "fixture",
        )
        .expect("action should enqueue");
    let deadline = Instant::now() + Duration::from_secs(1);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "fixture action should start");
        std::thread::yield_now();
    }

    let started_at = Instant::now();
    drop(coordinator);
    assert!(started_at.elapsed() < Duration::from_secs(1));
    owner.shutdown();
    let _ = std::fs::remove_file(marker);
}

#[test]
fn coordinator_bounds_outstanding_action_completions_without_dropping_them() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register("fixture.extension", extension, search)
        .expect("worker should register");

    for index in 0..16 {
        coordinator
            .invoke(
                "fixture.extension",
                u64::try_from(index).expect("generation should fit"),
                "fixture.entry",
                "fixture.run",
                "fixture",
            )
            .expect("bounded action should enqueue");
    }
    assert!(matches!(
        coordinator.invoke(
            "fixture.extension",
            17,
            "fixture.entry",
            "fixture.run",
            "fixture",
        ),
        Err(SupervisorError::QueueFull)
    ));

    drop(coordinator);
    owner.shutdown();
}

#[test]
fn initialization_timeout_terminates_the_child() {
    let limits = ExtensionLimits {
        handshake_timeout: Duration::from_millis(50),
        ..ExtensionLimits::default()
    };
    let mut extension =
        ExtensionProcess::spawn_with(fixture_path(), ["--hang-initialize".into()], limits)
            .expect("fixture should spawn");
    assert!(matches!(
        extension.initialize("initialize-timeout"),
        Err(SupervisorError::Timeout("initialization"))
    ));
    assert!(
        extension
            .try_wait()
            .expect("process status should be available")
            .is_some(),
        "initialization timeout should reap the child"
    );
}

#[test]
fn crashed_extension_restarts_with_a_fixed_budget() {
    let limits = ExtensionLimits {
        max_restarts: 1,
        ..ExtensionLimits::default()
    };
    let mut extension =
        ExtensionProcess::spawn_with(fixture_path(), ["--exit-after-initialize".into()], limits)
            .expect("fixture should spawn");
    extension
        .initialize("initialize-before-crash")
        .expect("fixture should initialize");
    wait_for_exit(&mut extension);
    assert!(
        extension
            .recover_if_exited("initialize-after-restart")
            .expect("first restart should succeed")
    );
    wait_for_exit(&mut extension);
    assert!(matches!(
        extension.recover_if_exited("restart-limit"),
        Err(SupervisorError::RestartLimit)
    ));
}

#[test]
fn stderr_is_drained_into_a_bounded_tail() {
    let limits = ExtensionLimits {
        stderr_tail_bytes: 128,
        ..ExtensionLimits::default()
    };
    let mut extension =
        ExtensionProcess::spawn_with(fixture_path(), ["--write-stderr".into()], limits)
            .expect("fixture should spawn");
    extension
        .initialize("initialize-stderr")
        .expect("fixture should initialize");
    let deadline = Instant::now() + Duration::from_secs(1);
    while extension.stderr_tail().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    let stderr = extension.stderr_tail();
    assert!(!stderr.is_empty());
    assert!(stderr.len() <= 128);
    extension
        .shutdown("shutdown-stderr")
        .expect("fixture should shut down");
}

fn wait_for_exit(extension: &mut ExtensionProcess) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while extension.try_wait().expect("process status").is_none() {
        assert!(Instant::now() < deadline, "fixture did not exit");
        std::thread::sleep(Duration::from_millis(1));
    }
}

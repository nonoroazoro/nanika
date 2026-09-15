use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use nanika_extension_package::ExtensionContributions;
use nanika_host::{
    ExtensionLimits, ExtensionProcess, ExtensionSearchCoordinator, SupervisorError,
    publish_extension_snapshot,
};
use nanika_search::{SearchOwner, UsageMap};

#[path = "support/TestHostServices.rs"]
mod test_host_services;

use test_host_services::TestHostServices;

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
fn fixture_acknowledges_configuration_through_the_supervised_protocol() {
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    extension
        .initialize("initialize-configuration")
        .expect("fixture should initialize");

    extension
        .apply_configuration("apply-configuration", Default::default())
        .expect("fixture configuration should apply");
    extension
        .shutdown("shutdown-configuration")
        .expect("fixture should shut down");
}

#[test]
fn uncorrelated_extension_error_fails_the_waiting_operation() {
    let mut extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--error-after-initialize".into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    extension
        .initialize("initialize-background-error")
        .expect("fixture should initialize");

    let error = extension
        .apply_configuration("apply-after-background-error", Default::default())
        .expect_err("uncorrelated errors must not leave configuration pending");

    assert!(error.to_string().contains("without a request id"));
}

#[test]
fn protocol_operations_require_initialization() {
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    let error = extension
        .query("query-before-initialize", 1, "fixture")
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
        .query("query-1", 7, "calculator")
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
fn extension_invocation_uses_the_common_host_service_boundary() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--request-launch-on-invoke".into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    let services = Arc::new(TestHostServices::new());
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator.set_host_services(services.clone());
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
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
    while services.request_count() == 0 {
        assert!(Instant::now() < deadline, "host service should be called");
        std::thread::yield_now();
    }
    drop(coordinator);
    owner.shutdown();
}

#[test]
fn fixture_completes_a_generation_tagged_refresh() {
    let mut extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    extension
        .initialize("initialize-refresh")
        .expect("fixture should initialize");
    extension
        .refresh("refresh-1", 9)
        .expect("fixture refresh should complete");
    extension
        .shutdown("shutdown-refresh")
        .expect("fixture should shut down");
}

#[test]
fn coordinator_dispatches_refresh_off_the_caller_thread() {
    let marker =
        std::env::temp_dir().join(format!("nanika-extension-refresh-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--mark-refresh={}", marker.display()).into()],
        ExtensionLimits::default(),
    )
    .expect("fixture should spawn");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
        .expect("worker should register");

    coordinator
        .refresh("fixture.extension", 9)
        .expect("refresh should enqueue");

    let deadline = Instant::now() + Duration::from_secs(1);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "refresh should reach extension");
        std::thread::yield_now();
    }
    drop(coordinator);
    owner.shutdown();
    let _ = std::fs::remove_file(marker);
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
        .query("query-search-owner", generation, "calculator")
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
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
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
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
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
        .register(
            "fixture.extension",
            extension,
            search,
            fixture_contributions(),
        )
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

fn fixture_contributions() -> ExtensionContributions {
    ExtensionContributions {
        root_search: Some(nanika_extension_package::RootSearchContribution {}),
        ..ExtensionContributions::default()
    }
}

#[path = "support/PendingHostService.rs"]
mod pending_host_service;

fn wait_for_review_condition(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "extension condition did not arrive"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn cancellation_marker(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nanika-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn superseded_query_error_is_drained_before_the_current_query() {
    let marker = cancellation_marker("query-cancellation");
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--cancel-query={}", marker.display()).into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let mut coordinator = ExtensionSearchCoordinator::new();
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
        .unwrap();
    let first = search.begin_query("blocked").unwrap();
    coordinator.query(first, "blocked");
    wait_for_review_condition(|| marker.exists());
    let latest = search
        .begin_query_with_expected_extensions("latest", ["fixture.extension".to_owned()])
        .unwrap();
    coordinator.query(latest, "latest");
    wait_for_review_condition(|| {
        search
            .latest_snapshot()
            .is_some_and(|snapshot| snapshot.generation == latest)
    });
    let snapshot = search.latest_snapshot().unwrap();
    coordinator.shutdown();
    std::fs::remove_file(marker).unwrap();
    assert_eq!(snapshot.results.len(), 1);
    assert_eq!(snapshot.results[0].candidate.title(), "latest");
}

#[test]
fn cancellation_uses_the_actual_terminal_result_and_unique_invocation_ids() {
    let marker = cancellation_marker("invocation-correlation");
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [
            format!("--cancel-invoke={}", marker.display()).into(),
            "--complete-on-cancel".into(),
        ],
        ExtensionLimits::default(),
    )
    .unwrap();
    let mut coordinator = ExtensionSearchCoordinator::new();
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
        .unwrap();
    let first = coordinator
        .invoke(
            "fixture.extension",
            7,
            "fixture.entry",
            "fixture.run",
            "query",
        )
        .unwrap();
    wait_for_review_condition(|| marker.exists());
    let first_request = std::fs::read_to_string(&marker).unwrap();
    coordinator
        .cancel_invocation("fixture.extension", 1)
        .unwrap();
    let first_result = first.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    let second = coordinator
        .invoke(
            "fixture.extension",
            7,
            "fixture.entry",
            "fixture.run",
            "query",
        )
        .unwrap();
    wait_for_review_condition(|| {
        std::fs::read_to_string(&marker).is_ok_and(|request| request != first_request)
    });
    let premature_result = second.recv_timeout(Duration::from_millis(100));
    coordinator
        .cancel_invocation("fixture.extension", 2)
        .unwrap();
    let second_result = second.recv_timeout(Duration::from_secs(5));
    coordinator.shutdown();
    std::fs::remove_file(marker).unwrap();
    assert!(
        matches!(
            first_result,
            nanika_host::ExtensionInvocationOutcome::Completed { .. }
        ),
        "a completion that wins cancellation remains completed"
    );
    assert!(
        matches!(
            premature_result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ),
        "duplicate old result must not complete the second action"
    );
    assert!(matches!(
        second_result,
        Ok(Ok(
            nanika_host::ExtensionInvocationOutcome::Completed { .. }
        ))
    ));
}

#[test]
fn accepted_host_service_result_survives_action_cancellation() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let services = Arc::new(pending_host_service::PendingHostService::default());
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--request-launch-on-invoke".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let mut coordinator = ExtensionSearchCoordinator::new();
    coordinator.set_host_services(services.clone());
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
        .unwrap();
    let invocation = coordinator
        .invoke(
            "fixture.extension",
            1,
            "fixture.entry",
            "fixture.run",
            "query",
        )
        .unwrap();
    wait_for_review_condition(|| services.submitted());
    coordinator
        .cancel_invocation("fixture.extension", 1)
        .unwrap();
    let premature_result = invocation.recv_timeout(Duration::from_millis(100));
    let delivered = services.complete();
    let result = invocation.recv_timeout(Duration::from_secs(5));
    coordinator.shutdown();
    assert!(matches!(
        premature_result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    assert!(
        delivered,
        "host service response receiver must remain alive"
    );
    assert!(matches!(
        result,
        Ok(Ok(
            nanika_host::ExtensionInvocationOutcome::Completed { .. }
        ))
    ));
}

#[test]
fn invocation_admission_waits_for_capacity_and_queued_cancellation_does_not_execute() {
    let marker = cancellation_marker("admission");
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--cancel-invoke={}", marker.display()).into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let mut coordinator = ExtensionSearchCoordinator::new();
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
        .unwrap();
    let first = coordinator
        .invoke(
            "fixture.extension",
            1,
            "fixture.entry",
            "fixture.run",
            "query",
        )
        .unwrap();
    wait_for_review_condition(|| marker.exists());
    let first_request = std::fs::read_to_string(&marker).unwrap();
    let mut queued = Vec::new();
    for _ in 0..16 {
        queued.push(
            coordinator
                .invoke(
                    "fixture.extension",
                    1,
                    "fixture.entry",
                    "fixture.run",
                    "query",
                )
                .unwrap(),
        );
    }
    for id in 2..=17 {
        coordinator
            .cancel_invocation("fixture.extension", id)
            .unwrap();
    }
    let coordinator = Arc::new(coordinator);
    let submitting = Arc::clone(&coordinator);
    let (admitted, admission) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        admitted
            .send(submitting.invoke(
                "fixture.extension",
                1,
                "fixture.entry",
                "fixture.run",
                "query",
            ))
            .unwrap();
    });
    let premature_admission = admission.recv_timeout(Duration::from_millis(100));
    coordinator
        .cancel_invocation("fixture.extension", 1)
        .unwrap();
    let first_result = first.recv_timeout(Duration::from_secs(5)).unwrap();
    let last = admission
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    thread.join().unwrap();
    let queued_results = queued
        .into_iter()
        .map(|response| response.recv_timeout(Duration::from_secs(5)))
        .collect::<Vec<_>>();
    wait_for_review_condition(|| {
        std::fs::read_to_string(&marker).is_ok_and(|request| request != first_request)
    });
    let last_request = std::fs::read_to_string(&marker).unwrap();
    let mut coordinator = Arc::try_unwrap(coordinator).ok().unwrap();
    coordinator.shutdown();
    let last_result = last.recv_timeout(Duration::from_secs(5));
    std::fs::remove_file(marker).unwrap();
    assert!(matches!(
        premature_admission,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    assert!(matches!(
        first_result,
        Ok(nanika_host::ExtensionInvocationOutcome::Cancelled)
    ));
    assert!(queued_results.iter().all(|result| matches!(
        result,
        Ok(Ok(nanika_host::ExtensionInvocationOutcome::Cancelled))
    )));
    assert!(
        last_request.ends_with("-18"),
        "cancelled queued work must never reach the extension"
    );
    assert!(matches!(
        last_result,
        Ok(Ok(nanika_host::ExtensionInvocationOutcome::Cancelled))
    ));
}

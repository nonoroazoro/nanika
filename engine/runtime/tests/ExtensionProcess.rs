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
    extension.shutdown().expect("fixture should shut down");
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
    extension.shutdown().expect("fixture should shut down");
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
    extension.shutdown().expect("fixture should shut down");
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
    extension.shutdown().expect("fixture should shut down");
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
    let coordinator = ExtensionSearchCoordinator::default();
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
            coordinator.instance_id("fixture.extension").unwrap(),
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
    extension.shutdown().expect("fixture should shut down");
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
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
        .expect("worker should register");

    let completion = coordinator
        .refresh("fixture.extension", 9)
        .expect("refresh should enqueue");

    completion
        .recv_timeout(Duration::from_secs(2))
        .expect("refresh should acknowledge completion")
        .expect("refresh should succeed");

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
    extension.shutdown().expect("fixture should shut down");
    owner.shutdown();
}

#[test]
fn queued_refreshes_wait_for_their_own_completion() {
    let root = cancellation_marker("refresh-completion");
    std::fs::create_dir_all(&root).unwrap();
    let blocker = root.join("refresh-fixture.extension-1.block");
    std::fs::write(&blocker, b"wait").unwrap();
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [format!("--data-root={}", root.display()).into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            owner.handle(),
            fixture_contributions(),
        )
        .unwrap();
    let first = coordinator.refresh("fixture.extension", 9).unwrap();
    let second = coordinator.refresh("fixture.extension", 9).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let entered = root.join("refresh-fixture.extension-1.entered");
    while !entered.exists() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    // Always release the fixture before assertions so regressions cannot strand it.
    let was_entered = entered.exists();
    let first_pending = matches!(first.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));
    let second_pending = matches!(second.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));
    std::fs::remove_file(blocker).unwrap();
    let first_result = first.recv_timeout(Duration::from_secs(3));
    let second_result = second.recv_timeout(Duration::from_secs(3));
    drop(coordinator);
    owner.shutdown();
    std::fs::remove_dir_all(root).unwrap();
    assert!(was_entered && first_pending && second_pending);
    first_result.unwrap().unwrap();
    second_result.unwrap().unwrap();
}

#[test]
fn root_refresh_isolates_failures_and_skips_static_contributors() {
    let root = cancellation_marker("refresh-isolation");
    std::fs::create_dir_all(&root).unwrap();
    let refreshed = root.join("healthy");
    let skipped = root.join("static");
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .refresh_root_search(1)
        .expect("zero-extension host refresh succeeds");
    for (id, argument, contributions) in [
        (
            "test.failed",
            "--fail-refresh".to_owned(),
            fixture_contributions(),
        ),
        (
            "test.healthy",
            format!("--mark-refresh={}", refreshed.display()),
            fixture_contributions(),
        ),
        (
            "test.static",
            format!("--mark-refresh={}", skipped.display()),
            nanika_extension_package::ExtensionContributions::default(),
        ),
    ] {
        let extension = ExtensionProcess::spawn_with(
            fixture_path(),
            [argument.into()],
            ExtensionLimits::default(),
        )
        .unwrap();
        coordinator
            .register(id, extension, owner.handle(), contributions)
            .unwrap();
    }
    let result = coordinator.refresh_root_search(1);
    drop(coordinator);
    owner.shutdown();
    let healthy_completed = refreshed.exists();
    let static_skipped = !skipped.exists();
    std::fs::remove_dir_all(root).unwrap();
    let error = result.expect_err("refresh failure must propagate");
    assert!(error.contains("test.failed") && error.contains("fixture refresh failed"));
    assert!(healthy_completed && static_skipped);
}

#[test]
fn extension_search_worker_dispatches_off_the_caller_thread() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let extension = ExtensionProcess::spawn(fixture_path()).expect("fixture should spawn");
    let coordinator = ExtensionSearchCoordinator::default();
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
    let coordinator = ExtensionSearchCoordinator::default();
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
    let coordinator = ExtensionSearchCoordinator::default();
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
            coordinator.instance_id("fixture.extension").unwrap(),
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
    extension.shutdown().expect("fixture should shut down");
}

fn fixture_contributions() -> ExtensionContributions {
    ExtensionContributions {
        root_search: Some(nanika_extension_package::RootSearchContribution::default()),
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
    let coordinator = ExtensionSearchCoordinator::new();
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
    let coordinator = ExtensionSearchCoordinator::new();
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
            coordinator.instance_id("fixture.extension").unwrap(),
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
            coordinator.instance_id("fixture.extension").unwrap(),
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
    let coordinator = ExtensionSearchCoordinator::new();
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
            coordinator.instance_id("fixture.extension").unwrap(),
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
    let coordinator = ExtensionSearchCoordinator::new();
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
            coordinator.instance_id("fixture.extension").unwrap(),
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
                    coordinator.instance_id("fixture.extension").unwrap(),
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
                submitting.instance_id("fixture.extension").unwrap(),
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
    let coordinator = Arc::try_unwrap(coordinator).ok().unwrap();
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

#[test]
fn query_completes_while_refresh_is_waiting_for_its_terminal_acknowledgement() {
    let marker = cancellation_marker("query-during-refresh");
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        [
            "--defer-refresh-until-query".into(),
            format!("--mark-refresh={}", marker.display()).into(),
        ],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
        .unwrap();
    let completion = coordinator.refresh("fixture.extension", 1).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    let started = marker.exists();
    let pending = completion.try_recv().is_err();
    let generation = search.begin_query("during scan").unwrap();
    coordinator.query(generation, "during scan");
    let refreshed = completion.recv_timeout(Duration::from_secs(3));
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut queried = false;
    while Instant::now() < deadline {
        if search.latest_snapshot().is_some_and(|snapshot| {
            snapshot.generation == generation && snapshot.results.len() == 1
        }) {
            queried = true;
            break;
        }
        std::thread::yield_now();
    }
    coordinator.shutdown();
    owner.shutdown();
    if marker.exists() {
        std::fs::remove_file(marker).unwrap();
    }
    assert!(started && pending && queried);
    refreshed.unwrap().unwrap();
}

#[test]
fn refresh_requeries_the_same_generation_with_an_entry_patch() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--patch-query".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
        .unwrap();
    let generation = search.begin_query("").unwrap();
    coordinator.query(generation, "");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !search
        .latest_snapshot()
        .is_some_and(|snapshot| snapshot.results.len() == 3)
    {
        assert!(Instant::now() < deadline, "baseline snapshot should arrive");
        std::thread::yield_now();
    }
    coordinator
        .refresh("fixture.extension", generation)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut updated = false;
    while Instant::now() < deadline {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.results.len() == 2
            && snapshot
                .results
                .iter()
                .any(|entry| entry.candidate.title() == "Updated")
            && snapshot
                .results
                .iter()
                .any(|entry| entry.candidate.title() == "Keep")
        {
            updated = true;
            break;
        }
        std::thread::yield_now();
    }
    coordinator.shutdown();
    owner.shutdown();
    assert!(
        updated,
        "patch must keep unrelated entries and remove only named identities"
    );
}

#[test]
fn cancelled_patch_requires_a_new_baseline_in_the_same_generation() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--cancel-patch-once".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
        .unwrap();
    let generation = search.begin_query("").unwrap();
    coordinator.query(generation, "");
    wait_for_review_condition(|| {
        search
            .latest_snapshot()
            .is_some_and(|s| s.results.iter().any(|e| e.candidate.title() == "Original"))
    });
    coordinator.query(generation, "");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut recovered = false;
    while Instant::now() < deadline {
        if search.latest_snapshot().is_some_and(|s| {
            s.generation == generation && s.results.iter().any(|e| e.candidate.title() == "Updated")
        }) {
            recovered = true;
            break;
        }
        std::thread::yield_now();
    }
    coordinator.shutdown();
    owner.shutdown();
    assert!(
        recovered,
        "discarded patches must be recovered before resuming incremental replies"
    );
}

#[test]
fn cumulative_catalog_patches_exceeding_eight_mib_remain_searchable() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let (sent, received) = std::sync::mpsc::channel();
    search.set_notifier(Arc::new(move || {
        let _ = sent.send(());
    }));
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--large-catalog".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            fixture_contributions(),
        )
        .unwrap();
    let generation = search.begin_query("").unwrap();
    coordinator.query(generation, "");
    for expected in [4000, 6001] {
        loop {
            received
                .recv_timeout(Duration::from_secs(10))
                .expect("complete catalog should arrive");
            if search
                .latest_snapshot()
                .is_some_and(|snapshot| snapshot.results.len() == expected)
            {
                break;
            }
        }
        if expected == 4000 {
            coordinator.query(generation, "");
        }
    }
    let snapshot = search.latest_snapshot().unwrap();
    assert_eq!(
        snapshot.results.first().unwrap().candidate.entry_id(),
        "entry-0"
    );
    assert_eq!(
        snapshot.results.last().unwrap().candidate.entry_id(),
        "entry-6000"
    );
    coordinator.shutdown();
    owner.shutdown();
}

#[test]
fn manifest_contributions_do_not_replace_an_incremental_catalog() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--patch-query".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    let mut contributions = fixture_contributions();
    contributions
        .commands
        .push(nanika_extension_package::CommandContribution {
            command: "fixture.command".into(),
            action: nanika_protocol::Action::primary(
                nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                "Run",
            ),
            title: "Manifest command".into(),
            description: "Static command".into(),
            category: None,
            keywords: Vec::new(),
            icon: None,
        });
    contributions
        .views
        .push(nanika_extension_package::ViewContribution {
            id: "fixture.view".into(),
            title: "Manifest view".into(),
            description: "Static view".into(),
            category: None,
            keywords: Vec::new(),
            icon: None,
        });
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            contributions,
        )
        .unwrap();
    let generation = search.begin_query("").unwrap();
    coordinator.query(generation, "");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !search
        .latest_snapshot()
        .is_some_and(|snapshot| snapshot.results.len() == 5)
    {
        assert!(Instant::now() < deadline, "baseline snapshot should arrive");
        std::thread::yield_now();
    }
    coordinator
        .refresh("fixture.extension", generation)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut updated = false;
    while Instant::now() < deadline {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.results.len() == 4
            && snapshot
                .results
                .iter()
                .any(|entry| entry.candidate.title() == "Updated")
            && snapshot
                .results
                .iter()
                .any(|entry| entry.candidate.title() == "Keep")
        {
            updated = true;
            break;
        }
        std::thread::yield_now();
    }
    coordinator.shutdown();
    owner.shutdown();
    assert!(
        updated,
        "patch must keep unrelated entries and remove only named identities"
    );
}

#[test]
fn manifest_identity_is_authoritative_when_a_patch_changes_its_action_id() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--patch-query".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    let mut contributions = fixture_contributions();
    contributions
        .commands
        .push(nanika_extension_package::CommandContribution {
            command: "fixture.entry".into(),
            action: nanika_protocol::Action::primary(
                nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                "Run",
            ),
            title: "Manifest command".into(),
            description: "Static command".into(),
            category: None,
            keywords: Vec::new(),
            icon: None,
        });
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            contributions,
        )
        .unwrap();
    let generation = search.begin_query("").unwrap();
    coordinator.query(generation, "");
    wait_for_review_condition(|| {
        search.latest_snapshot().is_some_and(|snapshot| {
            snapshot
                .results
                .iter()
                .any(|row| row.candidate.title() == "Keep")
        })
    });
    let baseline = search.latest_snapshot().unwrap();
    coordinator
        .refresh("fixture.extension", generation)
        .unwrap()
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
        .unwrap();
    wait_for_review_condition(|| {
        search.latest_snapshot().is_some_and(|snapshot| {
            !snapshot
                .results
                .iter()
                .any(|row| row.candidate.entry_id() == "fixture.remove")
        })
    });
    let updated = search.latest_snapshot().unwrap();
    coordinator.shutdown();
    owner.shutdown();
    assert_eq!(
        baseline.results.len(),
        3,
        "manifest identity must replace the conflicting dynamic identity"
    );
    assert_eq!(updated.results.len(), 2);
    let command = updated
        .results
        .iter()
        .find(|row| row.candidate.entry_id() == "fixture.entry")
        .unwrap();
    assert_eq!(command.candidate.title(), "Manifest command");
    assert_eq!(
        command.candidate.action_id(),
        nanika_protocol::COMMAND_EXECUTE_ACTION_ID
    );
}

#[test]
fn catalog_extensions_publish_once_and_search_without_query_messages() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    let extension = ExtensionProcess::spawn_with(
        fixture_path(),
        ["--catalog-only".into()],
        ExtensionLimits::default(),
    )
    .unwrap();
    let coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "fixture.extension",
            extension,
            search.clone(),
            ExtensionContributions {
                root_search: Some(nanika_extension_package::RootSearchContribution {
                    mode: nanika_extension_package::RootSearchMode::Catalog,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    for query in ["", "catalog", "entry", ""] {
        let generation = search
            .begin_query_with_expected_extensions(query, ["fixture.extension".into()])
            .unwrap();
        coordinator.query(generation, query);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !search.latest_snapshot().is_some_and(|snapshot| {
            snapshot.generation == generation && snapshot.results.len() == 1
        }) {
            assert!(
                Instant::now() < deadline,
                "catalog must remain searchable without extension queries"
            );
            std::thread::yield_now();
        }
    }
    coordinator.shutdown();
    owner.shutdown();
}

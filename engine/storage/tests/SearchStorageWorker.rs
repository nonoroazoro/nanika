use std::time::{Duration, Instant};

use nanika_search::{Candidate, CandidateKind, SearchOwner, UsageMap, normalize_history_key};

use crate::{HostDatabase, SearchStorageWorker, StorageQueueError, unix_timestamp};

#[test]
fn history_persists_with_punctuation_preserving_identity() {
    let root = std::env::temp_dir().join(format!("nanika-storage-search-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = root.join("nanika.db");
    let (worker, state) =
        SearchStorageWorker::spawn(&database).expect("storage owner should start");
    assert!(state.input_history.is_empty());
    for query in ["git --help", "git help", "C++", "C#"] {
        worker
            .record_history(normalize_history_key(query), query, unix_timestamp())
            .expect("history write should enqueue");
    }
    worker.shutdown();
    let reopened = HostDatabase::open(&database).expect("database should reopen");
    assert_eq!(reopened.load_input_history().expect("history").len(), 4);
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn persisted_usage_is_the_authority_for_in_memory_ranking() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-usage-{}.db", std::process::id()));
    cleanup(&database);
    let (worker, _) = SearchStorageWorker::spawn(&database).expect("storage owner should start");
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    worker.attach_search(search.clone());
    worker
        .register_builtin_extension("test.extension")
        .expect("extension registration should enqueue");
    let generation = search.begin_query("tool").expect("query should enqueue");
    search
        .register_extension("test.extension", 1)
        .unwrap()
        .publish_extension_snapshot(
            generation,
            vec![
                Candidate::new(
                    CandidateKind::Action,
                    "test.extension",
                    "a",
                    "Tool",
                    "open",
                    Vec::new(),
                    Vec::new(),
                ),
                Candidate::new(
                    CandidateKind::Action,
                    "test.extension",
                    "b",
                    "Tool",
                    "open",
                    Vec::new(),
                    Vec::new(),
                ),
            ],
            true,
        )
        .expect("snapshot should enqueue");
    worker
        .record_usage("test.extension", "b", "open", "tool", unix_timestamp())
        .expect("usage write should enqueue");
    worker.shutdown();

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot
                .results
                .first()
                .is_some_and(|result| result.candidate.entry_id() == "b")
        {
            break;
        }
        assert!(Instant::now() < deadline, "persisted usage should rerank");
        std::thread::yield_now();
    }
    let reopened = HostDatabase::open(&database).expect("database should reopen");
    assert_eq!(reopened.load_usage().expect("usage").len(), 1);
    drop(reopened);
    owner.shutdown();
    cleanup(&database);
}

#[test]
fn invalid_extension_ids_are_rejected_before_enqueueing() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-invalid-{}.db", std::process::id()));
    cleanup(&database);
    let (worker, _) = SearchStorageWorker::spawn(&database).expect("storage owner should start");
    assert!(matches!(
        worker.register_builtin_extension("../escape"),
        Err(StorageQueueError::InvalidExtensionId)
    ));
    worker.shutdown();
    cleanup(&database);
}

#[test]
fn operation_failures_return_the_source_and_remain_available_to_diagnostics() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-failure-{}.db", std::process::id()));
    cleanup(&database);
    let (worker, _) = SearchStorageWorker::spawn(&database).expect("storage owner should start");
    let connection = rusqlite::Connection::open(&database).expect("database should reopen");
    connection
        .execute("DROP TABLE input_history", [])
        .expect("history table should be removed");
    drop(connection);
    let result = worker
        .record_history("query", "query", unix_timestamp())
        .unwrap()
        .wait();
    assert!(
        matches!(&result, Err(StorageQueueError::Operation(source)) if source.contains("input_history"))
    );

    let failure = worker
        .last_failure()
        .expect("storage failure should remain available to diagnostics");

    assert_eq!(failure.operation(), "record input history");
    assert!(failure.source().contains("input_history"));
    assert!(failure.sequence() > 0);
    worker.shutdown();
    cleanup(&database);
}

#[test]
fn usage_is_preserved_until_an_explicit_reset() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-persistence-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).expect("database should open");
    host.register_builtin_extension("test.extension")
        .expect("extension should register");
    host.record_usage("test.extension", "old", "open", "old", 1)
        .expect("old usage should persist");
    host.record_usage("test.extension", "new", "open", "new", u64::MAX)
        .expect("new usage should persist");
    let usage = host.load_usage().expect("usage should load");
    assert_eq!(usage.len(), 2);
    host.reset_usage().expect("usage should reset");
    assert!(host.load_usage().expect("usage should load").is_empty());
    drop(host);
    cleanup(&database);
}

#[test]
fn malformed_extension_metadata_is_isolated_from_storage_startup() {
    let root = std::env::temp_dir().join(format!(
        "nanika-storage-isolated-extension-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let database = root.join("nanika.db");
    let host = HostDatabase::open(&database).expect("database should open");
    host.register_builtin_extension("com.example.valid")
        .expect("valid extension should register");
    drop(host);
    let connection = rusqlite::Connection::open(&database).expect("raw database should open");
    connection
        .execute_batch("PRAGMA ignore_check_constraints=ON")
        .expect("corrupt fixture should bypass schema checks");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind
             ) VALUES ('com.example.invalid', 'corrupt')",
            [],
        )
        .expect("invalid fixture should be inserted");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind
             ) VALUES ('com.example.incomplete-package', 'external')",
            [],
        )
        .expect("incomplete package fixture should be inserted");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind
             ) VALUES ('../escape', 'external')",
            [],
        )
        .expect("invalid id fixture should be inserted");
    drop(connection);

    let (worker, state) =
        SearchStorageWorker::spawn(&database).expect("storage owner should still start");
    assert_eq!(state.extensions.len(), 1);
    assert_eq!(state.extensions[0].extension_id, "com.example.valid");
    assert_eq!(state.extension_errors.len(), 3);
    assert!(
        state
            .extension_errors
            .iter()
            .any(|error| error.contains("com.example.invalid"))
    );
    assert!(
        state
            .extension_errors
            .iter()
            .any(|error| error.contains("com.example.incomplete-package"))
    );
    assert!(
        state
            .extension_errors
            .iter()
            .any(|error| error.contains("../escape"))
    );
    worker.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

fn cleanup(database: &std::path::Path) {
    let _ = std::fs::remove_file(database);
    let _ = std::fs::remove_file(database.with_extension("db-wal"));
    let _ = std::fs::remove_file(database.with_extension("db-shm"));
}

#[test]
fn committed_execution_and_reset_survive_projection_failure() {
    let database =
        std::env::temp_dir().join(format!("nanika-commit-receipt-{}.db", std::process::id()));
    cleanup(&database);
    let (worker, _) = SearchStorageWorker::spawn(&database).unwrap();
    worker.register_builtin_extension("test.extension").unwrap();
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    worker.attach_search(owner.handle());
    owner.shutdown();
    worker
        .record_execution(
            "query",
            "Query",
            nanika_search::UsageKey::new("test.extension", "entry", "open", "query"),
            1,
            1,
        )
        .unwrap()
        .wait()
        .unwrap();
    let reader = HostDatabase::open(&database).unwrap();
    assert_eq!(reader.load_usage().unwrap().len(), 1);
    assert_eq!(reader.load_input_history().unwrap().len(), 1);
    worker.reset_usage().unwrap().wait().unwrap();
    assert!(reader.load_usage().unwrap().is_empty());
    assert_eq!(reader.load_input_history().unwrap().len(), 1);
    worker.shutdown();
    let failure = worker.last_failure().unwrap();
    assert_eq!(failure.operation(), "publish committed storage change");
    assert_eq!(failure.sequence(), 2);
    drop(reader);
    cleanup(&database);
}

#[test]
fn commit_receipt_precedes_projection_backpressure_and_shutdown_drains_accepted_writes() {
    use std::sync::{Arc, Mutex, mpsc};
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-backpressure-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let (worker, _) = SearchStorageWorker::spawn(&database).unwrap();
    worker
        .register_builtin_extension("test.extension")
        .unwrap()
        .wait()
        .unwrap();
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let search = owner.handle();
    worker.attach_search(search.clone());
    let (entered, observed) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let gate = Mutex::new(Some(released));
    search.set_notifier(Arc::new(move || {
        if let Some(gate) = gate.lock().unwrap().take() {
            entered.send(()).unwrap();
            gate.recv().unwrap();
        }
    }));
    search.begin_query("tool").unwrap();
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..16 {
        search.reset_persisted_usage().unwrap();
    }
    let commit = worker
        .record_usage("test.extension", "entry", "open", "tool", 1)
        .unwrap();
    let (done, receipt) = mpsc::sync_channel(1);
    let waiter = std::thread::spawn(move || {
        done.send(commit.wait()).unwrap();
    });
    let result = receipt.recv_timeout(Duration::from_secs(1));
    // While the owner is blocked publishing its first commit, further writes are
    // accepted without awaiting disk or search. Discarding receipts is intentional.
    for _ in 0..8 {
        drop(
            worker
                .record_usage("test.extension", "entry", "open", "tool", 2)
                .unwrap(),
        );
    }
    release.send(()).unwrap();
    waiter.join().unwrap();
    result
        .expect("durable receipt must not await a full search queue")
        .unwrap();
    worker.shutdown();
    owner.shutdown();
    let reader = HostDatabase::open(&database).unwrap();
    assert_eq!(reader.load_usage().unwrap()[0].execution_count, 9);
    drop(reader);
    cleanup(&database);
}

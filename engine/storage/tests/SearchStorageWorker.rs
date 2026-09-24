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
        .register_builtin_extension("test.extension", unix_timestamp())
        .expect("extension registration should enqueue");
    let generation = search.begin_query("tool").expect("query should enqueue");
    search
        .publish_extension_snapshot(
            "test.extension",
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
    assert_eq!(
        worker.register_builtin_extension("../escape", unix_timestamp()),
        Err(StorageQueueError::InvalidExtensionId)
    );
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
    let result = worker.record_history("query", "query", unix_timestamp());
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
    host.register_builtin_extension("test.extension", 1)
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
    host.register_builtin_extension("com.example.valid", 1)
        .expect("valid extension should register");
    drop(host);
    let connection = rusqlite::Connection::open(&database).expect("raw database should open");
    connection
        .execute_batch("PRAGMA ignore_check_constraints=ON")
        .expect("corrupt fixture should bypass schema checks");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, updated_at
             ) VALUES ('com.example.invalid', 'corrupt', 1)",
            [],
        )
        .expect("invalid fixture should be inserted");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, updated_at
             ) VALUES ('com.example.incomplete-package', 'external', 1)",
            [],
        )
        .expect("incomplete package fixture should be inserted");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, updated_at
             ) VALUES ('../escape', 'external', 1)",
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

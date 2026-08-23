use std::time::{Duration, Instant};

use nanika_search::{
    Candidate, SearchOwner, USAGE_RETENTION_DAYS, UsageMap, normalize_history_key,
};

use crate::{ExtensionKind, HostDatabase, SearchStorageWorker, StorageQueueError, unix_timestamp};

#[test]
fn history_persists_with_punctuation_preserving_identity() {
    let root = std::env::temp_dir().join(format!("nanika-storage-search-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let database = root.join("nanika.db");
    let (worker, state) =
        SearchStorageWorker::spawn(&database, 10).expect("storage owner should start");
    assert!(state.input_history.is_empty());
    for query in ["git --help", "git help", "C++", "C#"] {
        worker
            .record_history(normalize_history_key(query), query, unix_timestamp())
            .expect("history write should enqueue");
    }
    worker.shutdown();
    let reopened = HostDatabase::open(&database).expect("database should reopen");
    assert_eq!(reopened.load_input_history(10).expect("history").len(), 4);
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn persisted_usage_is_the_authority_for_in_memory_ranking() {
    let database =
        std::env::temp_dir().join(format!("nanika-storage-usage-{}.db", std::process::id()));
    cleanup(&database);
    let (worker, _) =
        SearchStorageWorker::spawn(&database, 50).expect("storage owner should start");
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    worker.attach_search(search.clone());
    worker
        .register_extension("test.extension", ExtensionKind::External, unix_timestamp())
        .expect("extension registration should enqueue");
    let generation = search.begin_query("tool").expect("query should enqueue");
    search
        .publish_extension_snapshot(
            "test.extension",
            generation,
            vec![
                Candidate::new("test.extension", "a", "Tool", "open", Vec::new()),
                Candidate::new("test.extension", "b", "Tool", "open", Vec::new()),
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
    let (worker, _) =
        SearchStorageWorker::spawn(&database, 50).expect("storage owner should start");
    assert_eq!(
        worker.register_extension("../escape", ExtensionKind::External, unix_timestamp()),
        Err(StorageQueueError::InvalidExtensionId)
    );
    worker.shutdown();
    cleanup(&database);
}

#[test]
fn usage_retention_and_reset_remove_persisted_rows() {
    let database = std::env::temp_dir().join(format!(
        "nanika-storage-retention-{}.db",
        std::process::id()
    ));
    cleanup(&database);
    let host = HostDatabase::open(&database).expect("database should open");
    host.register_extension("test.extension", ExtensionKind::External, 1)
        .expect("extension should register");
    host.record_usage("test.extension", "old", "open", "old", 1)
        .expect("old usage should persist");
    let now = USAGE_RETENTION_DAYS * 86_400 + 2;
    host.record_usage("test.extension", "new", "open", "new", now)
        .expect("new usage should persist");
    let usage = host.load_usage().expect("usage should load");
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].entry_id, "new");
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
    host.register_extension("com.example.valid", ExtensionKind::External, 1)
        .expect("valid extension should register");
    drop(host);
    let connection = rusqlite::Connection::open(&database).expect("raw database should open");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, state, health, updated_at
             ) VALUES ('com.example.invalid', 'corrupt', 'enabled', 'healthy', 1)",
            [],
        )
        .expect("invalid fixture should be inserted");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, state, health, updated_at
             ) VALUES ('com.example.invalid-state', 'external', 1, 'healthy', 1)",
            [],
        )
        .expect("invalid state fixture should be inserted");
    connection
        .execute(
            "INSERT INTO extensions (
                extension_id, kind, state, health, updated_at
             ) VALUES ('../escape', 'external', 'enabled', 'healthy', 1)",
            [],
        )
        .expect("invalid id fixture should be inserted");
    drop(connection);

    let (worker, state) =
        SearchStorageWorker::spawn(&database, 50).expect("storage owner should still start");
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
            .any(|error| error.contains("com.example.invalid-state"))
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

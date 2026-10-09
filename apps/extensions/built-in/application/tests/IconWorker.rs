use super::*;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn cached_entries_publish_before_a_slow_native_icon_and_unrequested_entries_stay_empty() {
    let entries = Arc::new(RwLock::new(
        ["slow", "cached", "other"]
            .map(|id| (id.to_owned(), _entry(id)))
            .into_iter()
            .collect::<HashMap<_, _>>()
    ));
    let (events, receiver) = mpsc::sync_channel(8);
    let (started, native_started) = mpsc::sync_channel(0);
    let (finish, native_finish) = mpsc::sync_channel(0);
    let mut worker = IconWorker::_spawn(
        Arc::clone(&entries),
        events,
        |entry| {
            assert_ne!(entry.entry_id, "other");
            (entry.entry_id == "cached").then(|| IconReference::new(&entry.icon_key).unwrap())
        },
        move |entry| {
            assert_eq!(entry.entry_id, "slow");
            started.send(()).unwrap();
            native_finish.recv().unwrap();
            _ready(entry)
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["slow".into(), "cached".into()]);
    assert_eq!(_receive(&receiver), ["cached"]);
    native_started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(receiver.try_recv().is_err());
    finish.send(()).unwrap();
    assert_eq!(_receive(&receiver), ["slow"]);
    worker.stop();
    worker.join().unwrap();
    assert!(entries.read().unwrap()["other"]._icon.is_none());
}

#[test]
fn viewport_replacement_during_extraction_skips_old_pending_entries_and_keeps_its_wake() {
    let entries = Arc::new(RwLock::new(
        ["a", "b", "c"]
            .map(|id| (id.to_owned(), _entry(id)))
            .into_iter()
            .collect::<HashMap<_, _>>()
    ));
    let (events, receiver) = mpsc::sync_channel(8);
    let (started, native_started) = mpsc::sync_channel(0);
    let (finish, native_finish) = mpsc::sync_channel(0);
    let mut worker = IconWorker::_spawn(
        Arc::clone(&entries),
        events,
        |_| None,
        move |entry| {
            assert_ne!(entry.entry_id, "b");
            if entry.entry_id == "a" {
                started.send(()).unwrap();
                native_finish.recv().unwrap();
            }
            _ready(entry)
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into(), "b".into()]);
    native_started.recv_timeout(Duration::from_secs(5)).unwrap();
    worker.prepare_entries(2, vec!["c".into()]);
    worker.prepare_entries(1, vec!["b".into()]);
    finish.send(()).unwrap();
    assert_eq!(_receive(&receiver), ["a"]);
    assert_eq!(_receive(&receiver), ["c"]);
    worker.stop();
    worker.join().unwrap();
    assert!(entries.read().unwrap()["b"]._icon.is_none());
}

#[test]
fn deleted_or_replaced_sources_reject_stale_results_without_overwriting_new_metadata() {
    let entries = RwLock::new(HashMap::from([("a".into(), _entry("a"))]));
    let (events, receiver) = mpsc::sync_channel(8);
    let original = _entry("a");
    let prepared = _ready(&original);
    entries.write().unwrap().get_mut("a").unwrap().icon_key = "replacement".into();
    assert!(_publish(
        &entries,
        &events,
        vec![(original.clone(), prepared.clone())]
    ));
    assert!(receiver.try_recv().is_err());
    entries.write().unwrap().remove("a");
    assert!(_publish(
        &entries,
        &events,
        vec![(original.clone(), prepared.clone())]
    ));
    assert!(receiver.try_recv().is_err());
    let mut renamed = original.clone();
    renamed.display_name = "Renamed".into();
    entries.write().unwrap().insert("a".into(), renamed);
    assert!(_publish(&entries, &events, vec![(original, prepared)]));
    assert_eq!(_receive(&receiver), ["a"]);
    assert_eq!(entries.read().unwrap()["a"].display_name, "Renamed");
}

#[test]
fn discovery_wakes_a_viewport_requested_before_its_entries_exist() {
    let entries = Arc::new(RwLock::new(HashMap::new()));
    let (events, receiver) = mpsc::sync_channel(8);
    let mut worker = IconWorker::_spawn(Arc::clone(&entries), events, |_| None, _ready).unwrap();
    worker.prepare_entries(1, vec!["a".into()]);
    entries.write().unwrap().insert("a".into(), _entry("a"));
    let wake = worker.wake_handle();
    wake.0.lock().unwrap().wake();
    wake.1.notify_one();
    assert_eq!(_receive(&receiver), ["a"]);
    worker.stop();
    worker.join().unwrap();
}

#[test]
fn shutdown_finishes_the_active_icon_and_does_not_start_remaining_work() {
    let entries = Arc::new(RwLock::new(
        ["a", "b"]
            .map(|id| (id.to_owned(), _entry(id)))
            .into_iter()
            .collect::<HashMap<_, _>>()
    ));
    let (events, receiver) = mpsc::sync_channel(8);
    let (started, native_started) = mpsc::sync_channel(0);
    let (finish, native_finish) = mpsc::sync_channel(0);
    let mut worker = IconWorker::_spawn(
        entries,
        events,
        |_| None,
        move |entry| {
            assert_eq!(entry.entry_id, "a");
            started.send(()).unwrap();
            native_finish.recv().unwrap();
            _ready(entry)
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into(), "b".into()]);
    native_started.recv_timeout(Duration::from_secs(5)).unwrap();
    worker.stop();
    finish.send(()).unwrap();
    assert_eq!(_receive(&receiver), ["a"]);
    worker.join().unwrap();
    assert!(receiver.try_recv().is_err());
}

#[test]
fn failed_preparation_is_not_retried_by_duplicate_viewports() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let entries = Arc::new(RwLock::new(HashMap::from([("a".into(), _entry("a"))])));
    let attempts = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&attempts);
    let (events, receiver) = mpsc::sync_channel(1);
    let mut worker = IconWorker::_spawn(
        entries,
        events,
        |_| None,
        move |_| {
            count.fetch_add(1, Ordering::Relaxed);
            IconSource::Empty
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into()]);
    assert_eq!(_receive(&receiver), ["a"]);
    for generation in 2..100 {
        worker.prepare_entries(generation, vec!["a".into()]);
    }
    worker.stop();
    worker.join().unwrap();
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
}

#[test]
fn shutdown_can_drain_an_icon_publication_blocked_by_backpressure() {
    let entries = Arc::new(RwLock::new(
        ["a", "b", "c"]
            .map(|id| (id.to_owned(), _entry(id)))
            .into_iter()
            .collect::<HashMap<_, _>>()
    ));
    let (events, receiver) = mpsc::sync_channel(1);
    let (started, native_started) = mpsc::channel();
    let mut worker = IconWorker::_spawn(
        entries,
        events,
        |_| None,
        move |entry| {
            started.send(entry.entry_id.clone()).unwrap();
            _ready(entry)
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into(), "b".into(), "c".into()]);
    assert_eq!(
        native_started.recv_timeout(Duration::from_secs(5)).unwrap(),
        "a"
    );
    assert_eq!(
        native_started.recv_timeout(Duration::from_secs(5)).unwrap(),
        "b"
    );
    worker.stop();
    assert_eq!(_receive(&receiver), ["a"]);
    assert_eq!(_receive(&receiver), ["b"]);
    worker.join().unwrap();
    assert!(native_started.try_recv().is_err());
}

#[test]
fn identical_viewports_do_not_interrupt_work_but_catalog_changes_wake_it() {
    let mut priority = EntryPriority::default();
    priority.request(1, vec!["a".into(), "b".into()]);
    assert_eq!(priority.take(), ["a", "b"]);
    priority.request(1, vec!["a".into(), "b".into()]);
    assert!(!priority.pending());
    priority.wake();
    assert!(priority.pending());
    priority.take();
    priority.request(1, vec!["b".into(), "a".into()]);
    assert_eq!(priority.take(), ["b", "a"]);
    priority.request(0, vec!["obsolete".into()]);
    assert!(!priority.pending());
    priority.request(1, Vec::new());
    assert!(priority.take().is_empty());
}

#[test]
fn refresh_during_a_failed_native_attempt_retries_after_its_completion() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let entries = Arc::new(RwLock::new(HashMap::from([("a".into(), _entry("a"))])));
    let (events, receiver) = mpsc::sync_channel(8);
    let (started, native_started) = mpsc::sync_channel(0);
    let (finish, native_finish) = mpsc::sync_channel(0);
    let attempts = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&attempts);
    let mut worker = IconWorker::_spawn(
        Arc::clone(&entries),
        events,
        |_| None,
        move |entry| {
            if count.fetch_add(1, Ordering::Relaxed) == 0 {
                started.send(()).unwrap();
                native_finish.recv().unwrap();
                IconSource::Empty
            } else {
                _ready(entry)
            }
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into()]);
    native_started.recv_timeout(Duration::from_secs(5)).unwrap();
    let wake = worker.wake_handle();
    wake.0.lock().unwrap().retry_failed();
    wake.1.notify_one();
    finish.send(()).unwrap();
    assert_eq!(_receive(&receiver), ["a"]);
    assert_eq!(_receive(&receiver), ["a"]);
    worker.stop();
    worker.join().unwrap();
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    assert!(matches!(
        entries.read().unwrap()["a"]._icon.as_deref(),
        Some(nanika_protocol::IconSource::Cache(_))
    ));
}

#[test]
fn refresh_re_admits_offscreen_failures_and_preserves_successful_and_intentional_fallback_icons() {
    let mut unavailable = _entry("unavailable");
    unavailable._icon = Some(Arc::new(nanika_protocol::IconSource::Empty));
    let mut failed = _entry("failed");
    failed._icon = Some(Arc::new(nanika_protocol::IconSource::Cache(
        nanika_protocol::IconReference::new(IconCache::fallback_key()).unwrap()
    )));
    let mut generic = _entry("generic");
    generic.icon_key = IconCache::fallback_key().into();
    generic._icon = Some(Arc::new(_ready(&generic)));
    let mut successful = _entry("successful");
    successful._icon = Some(Arc::new(_ready(&successful)));
    let entries = RwLock::new(
        [unavailable, failed, generic.clone(), successful.clone()]
            .into_iter()
            .map(|entry| (entry.entry_id.clone(), entry))
            .collect()
    );
    _clear_failed_icons(&entries);
    let current = entries.read().unwrap();
    assert!(current["unavailable"]._icon.is_none());
    assert!(current["failed"]._icon.is_none());
    assert_eq!(current["generic"], generic);
    assert_eq!(current["successful"], successful);
}

#[test]
fn discovery_source_replacement_wakes_work_even_when_the_old_completion_is_rejected() {
    let entries = Arc::new(RwLock::new(HashMap::from([("a".into(), _entry("a"))])));
    let (events, receiver) = mpsc::sync_channel(8);
    let (started, native_started) = mpsc::sync_channel(0);
    let (finish, native_finish) = mpsc::sync_channel(0);
    let mut worker = IconWorker::_spawn(
        Arc::clone(&entries),
        events,
        |_| None,
        move |entry| {
            if entry.icon_key == "a" {
                started.send(()).unwrap();
                native_finish.recv().unwrap();
            }
            _ready(entry)
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into()]);
    native_started.recv_timeout(Duration::from_secs(5)).unwrap();
    entries.write().unwrap().get_mut("a").unwrap().icon_key = "replacement".into();
    let wake = worker.wake_handle();
    wake.0.lock().unwrap().wake();
    wake.1.notify_one();
    finish.send(()).unwrap();
    assert_eq!(_receive(&receiver), ["a"]);
    worker.stop();
    worker.join().unwrap();
    assert!(
        matches!(entries.read().unwrap()["a"]._icon.as_deref(), Some(nanika_protocol::IconSource::Cache(reference)) if reference.key() == "replacement")
    );
    assert!(receiver.try_recv().is_err());
}

#[test]
fn same_generation_viewport_reordering_changes_the_next_native_entry() {
    let entries = Arc::new(RwLock::new(
        ["a", "b", "c"]
            .map(|id| (id.to_owned(), _entry(id)))
            .into_iter()
            .collect::<HashMap<_, _>>()
    ));
    let (events, receiver) = mpsc::sync_channel(8);
    let (started, native_started) = mpsc::sync_channel(0);
    let (finish, native_finish) = mpsc::sync_channel(0);
    let mut worker = IconWorker::_spawn(
        entries,
        events,
        |_| None,
        move |entry| {
            if entry.entry_id == "a" {
                started.send(()).unwrap();
                native_finish.recv().unwrap();
            }
            _ready(entry)
        }
    )
    .unwrap();
    worker.prepare_entries(1, vec!["a".into(), "b".into(), "c".into()]);
    native_started.recv_timeout(Duration::from_secs(5)).unwrap();
    worker.prepare_entries(1, vec!["c".into(), "b".into(), "a".into()]);
    finish.send(()).unwrap();
    assert_eq!(_receive(&receiver), ["a"]);
    assert_eq!(_receive(&receiver), ["c"]);
    assert_eq!(_receive(&receiver), ["b"]);
    worker.stop();
    worker.join().unwrap();
    assert!(receiver.try_recv().is_err());
}

#[test]
fn stale_cached_completions_reject_source_path_and_resource_index_changes() {
    for change_path in [true, false] {
        let original = _entry("a");
        let prepared = _ready(&original);
        let mut replacement = original.clone();
        if change_path {
            replacement.icon_source = Some(crate::ApplicationIconSource::File {
                path: PathBuf::from("/replacement.exe"),
                index: 0
            });
        } else {
            replacement.icon_source = Some(crate::ApplicationIconSource::File {
                path: PathBuf::from("/a.exe"),
                index: 1
            });
        }
        let entries = RwLock::new(HashMap::from([("a".into(), replacement.clone())]));
        let (events, receiver) = mpsc::sync_channel(1);
        assert!(_publish(&entries, &events, vec![(original, prepared)]));
        assert_eq!(entries.read().unwrap()["a"], replacement);
        assert!(receiver.try_recv().is_err());
    }
}

fn _entry(id: &str) -> ApplicationEntry {
    ApplicationEntry::new(crate::ApplicationEntryData {
        entry_id: id.into(),
        source_key: id.into(),
        display_name: id.into(),
        normalized_name: id.into(),
        normalized_tokens: id.into(),
        launch_kind: "executable".into(),
        target_path: format!("/{id}.exe"),
        arguments_json: "{\"kind\":\"structured\",\"values\":[]}".into(),
        icon_key: id.into(),
        icon_source: Some(crate::ApplicationIconSource::File {
            path: PathBuf::from(format!("/{id}.exe")),
            index: 0
        }),
        priority: 0
    })
}

fn _ready(entry: &ApplicationEntry) -> IconSource {
    IconSource::Cache(IconReference::new(&entry.icon_key).unwrap())
}

fn _receive(events: &mpsc::Receiver<RuntimeEvent>) -> Vec<String> {
    match events.recv_timeout(Duration::from_secs(5)).unwrap() {
        RuntimeEvent::CatalogUpdated { entry_ids } => entry_ids,
        _ => panic!("expected icon publication")
    }
}

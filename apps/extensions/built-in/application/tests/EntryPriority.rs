use super::EntryPriority;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, mpsc};

#[test]
fn last_batch_keeps_the_wake_for_a_viewport_changed_during_extraction() {
    let priority = Arc::new(Mutex::new(EntryPriority::default()));
    assert!(priority.lock().unwrap().request(1, vec!["a".into()]));
    let (started, batch_started) = mpsc::sync_channel(0);
    let (finish, finish_batch) = mpsc::sync_channel(0);
    let worker_priority = Arc::clone(&priority);
    let worker = std::thread::spawn(move || {
        let batch = worker_priority.lock().unwrap().entries();
        assert_eq!(batch, ["a"]);
        started.send(()).unwrap();
        finish_batch.recv().unwrap();
        // A was already cached: completing it publishes no catalog change.
        // B must still get a wake, independently of any later host notification.
        let pending = HashSet::from(["b".to_owned()]);
        let mut priority = worker_priority.lock().unwrap();
        assert!(priority.finish(|ids| ids.iter().any(|id| pending.contains(id))));
        assert_eq!(priority.entries(), ["b"]);
        assert!(!priority.finish(|_| false));
    });
    batch_started.recv().unwrap();
    assert!(!priority.lock().unwrap().request(1, vec!["b".into()]));
    finish.send(()).unwrap();
    worker.join().unwrap();
    assert!(priority.lock().unwrap().request(1, vec!["c".into()]));
}

#[test]
fn newer_queries_and_empty_viewports_replace_queued_intent_without_duplicate_wakes() {
    let mut priority = EntryPriority::default();
    assert!(priority.request(2, vec!["a".into()]));
    assert!(!priority.request(1, vec!["obsolete".into()]));
    assert_eq!(priority.entries(), ["a"]);
    assert!(!priority.request(3, vec!["b".into()]));
    assert_eq!(priority.entries(), ["b"]);
    assert!(!priority.request(3, Vec::new()));
    assert!(!priority.finish(|ids| !ids.is_empty()));
    assert!(!priority.request(2, vec!["obsolete".into()]));
    assert!(priority.request(3, vec!["c".into()]));
    assert!(priority.finish(|ids| ids == ["c"]));
    assert!(!priority.request(3, vec!["c".into()]));
}

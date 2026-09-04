use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use nanika_extension_package::{CommandContribution, CommandMode, ExtensionContributions};

use crate::{
    ExtensionSearchState, ExtensionViewRequest, ExtensionViewRequestKind, ExtensionWork,
    contribution_candidates, next_work,
};

#[test]
fn view_event_wakes_an_idle_worker() {
    let state = Arc::new((Mutex::new(ExtensionSearchState::default()), Condvar::new()));
    let worker_state = Arc::clone(&state);
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let work = next_work(&worker_state);
        sender.send(work).expect("work result should send");
    });

    let (lock, ready) = &*state;
    lock.lock()
        .unwrap_or_else(|error| error.into_inner())
        .view_events
        .push_back(ExtensionViewRequest {
            request_id: 1,
            generation: 2,
            view_id: "test.view".to_owned(),
            revision: 3,
            kind: ExtensionViewRequestKind::Close,
        });
    ready.notify_one();

    let work = receiver
        .recv_timeout(Duration::from_millis(250))
        .expect("view event should wake the worker");
    assert!(matches!(work, Some(ExtensionWork::ViewEvent(_))));
    worker.join().expect("worker should stop");
}

#[test]
fn static_command_search_values_include_declared_metadata() {
    let candidates = contribution_candidates(&ExtensionContributions {
        commands: vec![CommandContribution {
            id: "example.open".to_owned(),
            title: "Open Example".to_owned(),
            description: "Open the example view.".to_owned(),
            mode: CommandMode::View,
            subtitle: Some("Example".to_owned()),
            keywords: vec!["sample".to_owned()],
        }],
        root_search: true,
    });

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].aliases,
        ["sample", "Open the example view.", "Example"]
    );
}

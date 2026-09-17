use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use nanika_extension_package::{
    CommandContribution, ContributionIcon, ExtensionContributions, RootSearchContribution,
    ViewContribution,
};

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
            completion: mpsc::channel().0,
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
            command: "example.open".to_owned(),
            title: "Open Example".to_owned(),
            description: "Open the example view.".to_owned(),
            category: Some("Example".to_owned()),
            keywords: vec!["sample".to_owned()],
            icon: Some(ContributionIcon::Clipboard),
        }],
        views: vec![ViewContribution {
            id: "example.view".to_owned(),
            title: "Example View".to_owned(),
            description: "Browse examples.".to_owned(),
            category: None,
            keywords: Vec::new(),
            icon: None,
        }],
        configuration: None,
        root_search: Some(RootSearchContribution {}),
    });

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].aliases,
        ["sample", "Open the example view.", "Example"]
    );
    assert_eq!(
        candidates[0].contribution_icon,
        Some(nanika_protocol::ContributionIcon::Clipboard)
    );
    assert_eq!(candidates[0].kind, nanika_protocol::CandidateKind::Action);
    assert_eq!(candidates[1].entry_id, "example.view");
    assert_eq!(candidates[1].kind, nanika_protocol::CandidateKind::View);
    assert_eq!(
        candidates[1].action_id,
        nanika_protocol::VIEW_OPEN_ACTION_ID
    );
}

#[test]
fn worker_exit_completes_every_queued_refresh_with_an_error() {
    let state = Arc::new((Mutex::new(ExtensionSearchState::default()), Condvar::new()));
    let mut completions = Vec::new();
    for request_id in 1..=2 {
        let (completion, receiver) = mpsc::sync_channel(1);
        state
            .0
            .lock()
            .unwrap()
            .refreshes
            .push_back(crate::ExtensionRefresh {
                request_id,
                generation: 1,
                completion,
            });
        completions.push(receiver);
    }
    drop(crate::ExtensionWorkerLifetime {
        extension_id: "test.extension".to_owned(),
        state,
        configuration_results: Arc::new(Mutex::new(Default::default())),
        notifier: Arc::new(Mutex::new(None)),
    });
    for completion in completions {
        let error = completion
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap_err();
        assert!(error.contains("closed before refreshing"));
    }
}

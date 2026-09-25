use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use nanika_extension_package::{
    CommandContribution, ExtensionContributions, RootSearchContribution, ViewContribution,
};

use crate::{
    ExtensionSearchState, ExtensionViewRequest, ExtensionViewRequestKind, ExtensionWork,
    contribution_candidates, next_work, queue_view_invalidation,
};

#[test]
fn view_invalidations_keep_only_the_latest_identity_per_extension() {
    let pending = Mutex::new(std::collections::HashMap::new());

    queue_view_invalidation(&pending, "extension.one", 1, "view.old".to_owned());
    queue_view_invalidation(&pending, "extension.one", 1, "view.current".to_owned());
    queue_view_invalidation(&pending, "extension.two", 1, "view.other".to_owned());

    let pending = pending.lock().unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending["extension.one"].view_id, "view.current");
    assert_eq!(pending["extension.two"].view_id, "view.other");
}

#[test]
fn failed_instance_rejects_further_work_and_leaves_restart_to_the_runtime_owner() {
    let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
    let search = owner.handle();
    let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let starts = Arc::clone(&attempts);
    let coordinator = crate::ExtensionSearchCoordinator::new();
    coordinator
        .register_source(
            "test.extension",
            crate::ExtensionRuntimeSource::Factory {
                activation: nanika_extension_package::ExtensionActivation::OnDemand,
                live_configuration: true,
                start: Box::new(move |_| {
                    starts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(std::io::Error::other("fixture activation failure"))
                }),
            },
            search.clone(),
            ExtensionContributions {
                commands: vec![CommandContribution {
                    action: nanika_protocol::Action::primary(
                        nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                        "Run",
                    ),
                    command: "test.command".to_owned(),
                    title: "Test".to_owned(),
                    description: "Test command".to_owned(),
                    category: None,
                    keywords: Vec::new(),
                    icon: None,
                }],
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(coordinator.ready_extension_ids(), ["test.extension"]);
    for _ in 0..2 {
        let error = match coordinator.invoke(
            "test.extension",
            coordinator.instance_id("test.extension").unwrap(),
            1,
            "test.command",
            "command.execute",
            "",
        ) {
            Ok(receipt) => receipt
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap_err(),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("fixture activation failure"));
    }
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(coordinator.ready_extension_ids().is_empty());
    coordinator.shutdown();
    owner.shutdown();
}

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
fn latest_visible_entry_hint_is_coalesced_behind_a_query() {
    let state = Arc::new((Mutex::new(ExtensionSearchState::default()), Condvar::new()));
    {
        let mut pending = state.0.lock().unwrap();
        pending.query = Some(crate::ExtensionSearchQuery {
            generation: 4,
            query: "mail".to_owned(),
        });
        pending.entry_preparation = Some((3, vec!["old".to_owned()]));
        pending.entry_preparation = Some((4, vec!["visible".to_owned()]));
    }
    assert!(matches!(next_work(&state), Some(ExtensionWork::Query(_))));
    assert!(matches!(
        next_work(&state),
        Some(ExtensionWork::PrepareEntries {
            generation: 4,
            entry_ids,
        }) if entry_ids == ["visible"]
    ));
}

#[test]
fn static_contribution_candidates_preserve_type_and_declared_metadata() {
    let candidates = contribution_candidates(&ExtensionContributions {
        commands: vec![CommandContribution {
            action: nanika_protocol::Action::primary(
                nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                "Run",
            ),
            command: "example.open".to_owned(),
            title: "Open Example".to_owned(),
            description: "Open the example view.".to_owned(),
            category: Some("Example".to_owned()),
            keywords: vec!["sample".to_owned()],
            icon: Some("assets/command.png".to_owned()),
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
        candidates[0].icon,
        Some(nanika_protocol::IconSource::Package {
            path: "assets/command.png".to_owned()
        })
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
        state,
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

#[test]
fn worker_panic_is_a_failed_stop() {
    let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
    let coordinator = crate::ExtensionSearchCoordinator::new();
    coordinator
        .register_source(
            "test.panic",
            crate::ExtensionRuntimeSource::Factory {
                activation: nanika_extension_package::ExtensionActivation::Startup,
                live_configuration: true,
                start: Box::new(|_| panic!("fixture factory panic")),
            },
            owner.handle(),
            Default::default(),
            Default::default(),
        )
        .unwrap();
    let worker = coordinator.worker("test.panic").unwrap();
    assert!(
        worker
            .wait_stopped()
            .unwrap_err()
            .contains("without a stop result")
    );
    assert_eq!(worker.lifecycle().0, crate::RuntimeExtensionState::Failed);
    coordinator.shutdown();
    owner.shutdown();
}

#[test]
fn dormant_factory_receives_the_last_accepted_configuration() {
    let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
    let coordinator = crate::ExtensionSearchCoordinator::new();
    let (observed, received) = mpsc::sync_channel(1);
    coordinator
        .register_source(
            "test.configuration",
            crate::ExtensionRuntimeSource::Factory {
                activation: nanika_extension_package::ExtensionActivation::OnDemand,
                live_configuration: true,
                start: Box::new(move |configuration| {
                    observed.send(configuration).unwrap();
                    Err(std::io::Error::other("fixture activation failure"))
                }),
            },
            owner.handle(),
            Default::default(),
            Default::default(),
        )
        .unwrap();
    let configuration = nanika_protocol::ExtensionConfiguration::new(
        [("value".into(), serde_json::json!(42))]
            .into_iter()
            .collect(),
    );
    let (completion, applied) = mpsc::sync_channel(1);
    assert!(
        coordinator
            .apply_configuration(
                "test.configuration",
                "configuration",
                configuration.clone(),
                false,
                Arc::new(|_| {}),
                completion
            )
            .unwrap()
    );
    assert!(matches!(
        applied
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap(),
        crate::ConfigurationApplication::Deferred
    ));
    assert!(received.try_recv().is_err());
    let instance = coordinator.instance_id("test.configuration").unwrap();
    let result = coordinator
        .invoke("test.configuration", instance, 1, "entry", "action", "")
        .unwrap();
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_err()
    );
    assert_eq!(
        received.recv_timeout(Duration::from_secs(2)).unwrap(),
        configuration
    );
    coordinator.shutdown();
    owner.shutdown();
}

use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::search_delivery::run_delivery;
use crate::{DesktopRuntime, SearchDelivery, SearchSession};

#[test]
fn one_unacknowledged_message_coalesces_updates_and_surfaces_startup_failure() {
    let (output, received) = mpsc::channel();
    let channel = tauri::ipc::Channel::new(move |body| {
        let tauri::ipc::InvokeResponseBody::Json(json) = body else {
            panic!("expected JSON")
        };
        output.send(json).unwrap();
        Ok(())
    });
    let shared = Arc::new(Mutex::new(DesktopRuntime {
        session: Some(SearchSession::new(1, channel)),
        ..Default::default()
    }));
    let (wakes, receiver) = mpsc::sync_channel(1);
    let worker = Arc::clone(&shared);
    let thread = std::thread::spawn(move || run_delivery(&worker, receiver));
    wakes.send(SearchDelivery::Wake).unwrap();
    let first = received.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(first.contains("\"phase\":\"searching\""));
    assert!(first.contains("\"sessionId\":1"));
    {
        let mut state = shared.lock().unwrap();
        state.startup_error = Some("internal startup failure".to_owned());
    }
    wakes.send(SearchDelivery::Wake).unwrap();
    assert!(received.recv_timeout(Duration::from_millis(50)).is_err());
    shared.lock().unwrap().session.as_mut().unwrap().in_flight = None;
    wakes.send(SearchDelivery::Wake).unwrap();
    let next = received.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(next.contains("\"phase\":\"error\""));
    assert!(next.contains("\"revision\":2"));
    assert!(!next.contains("internal startup failure"));
    assert!(!next.contains("\"results\""));
    assert!(!next.contains("\"current\""));
    wakes.send(SearchDelivery::Shutdown).unwrap();
    thread.join().unwrap();
}

#[test]
fn channel_callback_can_acknowledge_without_the_shared_state_lock() {
    let shared = Arc::new(Mutex::new(DesktopRuntime::default()));
    let callback_state = Arc::clone(&shared);
    let (sent, received) = mpsc::channel();
    let channel = tauri::ipc::Channel::new(move |_| {
        let mut state = callback_state
            .try_lock()
            .expect("transport must not own shared state");
        let session = state.session.as_mut().unwrap();
        assert_eq!(session.in_flight.unwrap().0, 1);
        session.in_flight = None;
        sent.send(()).unwrap();
        Ok(())
    });
    shared.lock().unwrap().session = Some(SearchSession::new(1, channel));
    let (wakes, receiver) = mpsc::sync_channel(1);
    let worker = Arc::clone(&shared);
    let thread = std::thread::spawn(move || run_delivery(&worker, receiver));
    wakes.send(SearchDelivery::Wake).unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        shared
            .lock()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .in_flight
            .is_none()
    );
    wakes.send(SearchDelivery::Shutdown).unwrap();
    thread.join().unwrap();
}

#[test]
fn navigation_only_payload_is_independent_of_unchanged_catalog_and_view_size() {
    let results = (0..2000)
        .map(|index| crate::SearchResult {
            allow_default_execution: true,
            extension_id: "test.extension".to_owned(),
            entry_id: format!("entry-{index}"),
            action_id: "open".to_owned(),
            title: format!("Application {index}"),
            subtitle: None,
            icon_url: None,
            contribution_icon: None,
            kind: "Extension".to_owned(),
            entry_type: "action",
        })
        .collect();
    let route = crate::ExtensionViewSnapshot {
        route_id: 1,
        extension_id: "test.extension".to_owned(),
        generation: 1,
        view_id: "view".to_owned(),
        revision: 1,
        view: Arc::new(nanika_protocol::View::Detail {
            detail: nanika_protocol::DetailView {
                title: None,
                content: nanika_protocol::DetailContent::Text {
                    value: "detail".repeat(16000),
                },
                metadata: Vec::new(),
                actions: Vec::new(),
            },
        }),
    };
    let mut update = crate::RootSearchSnapshot {
        navigation: crate::NavigationSnapshot {
            revision: 2,
            current: Some(Some(route)),
            busy: true,
            ..Default::default()
        },
        session_id: 1,
        request_id: 1,
        revision: 2,
        query: "".to_owned(),
        results: Some(results),
        phase: crate::SearchPhase::Ready,
        error: None,
        warnings: Vec::new(),
    };
    let (output, received) = mpsc::channel();
    let channel = tauri::ipc::Channel::new(move |body| {
        let tauri::ipc::InvokeResponseBody::Json(json) = body else {
            panic!("expected JSON")
        };
        output.send(json.len()).unwrap();
        Ok(())
    });
    channel.send(update.clone()).unwrap();
    let before = received.recv().unwrap();
    update.results = None;
    update.navigation.current = None;
    channel.send(update).unwrap();
    let after = received.recv().unwrap();
    assert!(before > 400_000);
    assert!(after < 300);
    println!(
        "2000 results + 96KB detail: full_payload_bytes={before}, navigation_only_bytes={after}"
    );
}

#[test]
fn stale_sessions_cannot_publish_or_acknowledge() {
    let session = SearchSession::new(2, tauri::ipc::Channel::new(|_| Ok(())));
    assert!(session.authorize(1).is_err());
    assert!(session.authorize(2).is_ok());
}

#[test]
fn preparation_completion_and_navigation_wakes_do_not_reschedule_preparation() {
    let root = std::env::temp_dir().join(format!(
        "nanika-delivery-feedback-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths =
        nanika_storage::NanikaPaths::from_roots(&root, root.join("cache"), root.join("config"));
    let runtime = Arc::new(nanika_host::RuntimeService::start(&paths, &[]).unwrap());
    let (wakes, receiver) = mpsc::sync_channel(1);
    let runtime_wakes = wakes.clone();
    runtime.set_notifier(Arc::new(move || {
        let _ = runtime_wakes.try_send(SearchDelivery::Wake);
    }));
    let (output, received) = mpsc::channel();
    let channel = tauri::ipc::Channel::new(move |body| {
        let tauri::ipc::InvokeResponseBody::Json(json) = body else {
            panic!("expected JSON")
        };
        output.send(json).unwrap();
        Ok(())
    });
    let mut session = SearchSession::new(1, channel);
    session.generation = runtime.begin_query("").unwrap();
    let shared = Arc::new(Mutex::new(DesktopRuntime {
        runtime: Some(Arc::clone(&runtime)),
        session: Some(session),
        ..Default::default()
    }));
    let (prepared, preparations) = mpsc::channel();
    let worker = Arc::clone(&shared);
    let completion_wakes = wakes.clone();
    let thread = std::thread::spawn(move || {
        crate::search_delivery::run_delivery_with_preparation(
            &worker,
            receiver,
            move |_, snapshot| {
                prepared.send(snapshot.generation).unwrap();
                // Reproduce the real worker's completion notification, including
                // its arrival before the WebView acknowledges the new snapshot.
                let _ = completion_wakes.try_send(SearchDelivery::Wake);
            },
        );
    });
    let _ = wakes.try_send(SearchDelivery::Wake);
    loop {
        let update = received.recv_timeout(Duration::from_secs(2)).unwrap();
        if update.contains("\"phase\":\"ready\"") {
            break;
        }
        shared.lock().unwrap().session.as_mut().unwrap().in_flight = None;
        let _ = wakes.try_send(SearchDelivery::Wake);
    }
    let first = preparations.recv_timeout(Duration::from_secs(1)).unwrap();
    let repeated_before_ack = preparations.recv_timeout(Duration::from_millis(50)).is_ok();
    {
        let mut state = shared.lock().unwrap();
        let session = state.session.as_mut().unwrap();
        session.in_flight = None;
        session.navigation.revision += 1;
    }
    wakes.send(SearchDelivery::Wake).unwrap();
    received.recv_timeout(Duration::from_secs(1)).unwrap();
    let repeated_for_navigation = preparations.recv_timeout(Duration::from_millis(50)).is_ok();
    let next = {
        let mut state = shared.lock().unwrap();
        let session = state.session.as_mut().unwrap();
        session.in_flight = None;
        session.delivered = None;
        session.generation = runtime.begin_query("new query").unwrap();
        session.generation
    };
    let _ = wakes.try_send(SearchDelivery::Wake);
    loop {
        let update = received.recv_timeout(Duration::from_secs(2)).unwrap();
        if update.contains("\"phase\":\"ready\"") {
            break;
        }
        shared.lock().unwrap().session.as_mut().unwrap().in_flight = None;
        let _ = wakes.try_send(SearchDelivery::Wake);
    }
    let next_prepared = preparations.recv_timeout(Duration::from_secs(1)).unwrap();
    wakes.send(SearchDelivery::Shutdown).unwrap();
    thread.join().unwrap();
    drop(shared);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        !repeated_before_ack,
        "completion must not create a feedback loop"
    );
    assert!(
        !repeated_for_navigation,
        "view updates must not prepare root entries again"
    );
    assert_ne!(first, next);
    assert_eq!(
        next_prepared, next,
        "a new query still prepares its results"
    );
}

#[test]
fn refresh_requires_the_current_root_session_and_excludes_pending_operations() {
    let mut session = SearchSession::new(2, tauri::ipc::Channel::new(|_| Ok(())));
    session.query = "keep this query".to_owned();
    assert!(session.begin_refresh(1).is_err());
    assert!(!session.navigation.busy);
    session.begin_refresh(2).expect("root session can refresh");
    assert!(session.begin_refresh(2).is_err());
    assert_eq!(session.query, "keep this query");
    session.navigation.finish(Ok(()));
    session.navigation.stack.push(crate::ExtensionViewSnapshot {
        route_id: 1,
        extension_id: "test.extension".to_owned(),
        generation: 1,
        view_id: "test.view".to_owned(),
        revision: 1,
        view: Arc::new(nanika_protocol::View::Detail {
            detail: nanika_protocol::DetailView {
                title: None,
                content: nanika_protocol::DetailContent::Text {
                    value: "content".to_owned(),
                },
                metadata: Vec::new(),
                actions: Vec::new(),
            },
        }),
    });
    assert!(session.begin_refresh(2).is_err());
    assert!(!session.navigation.busy);
}

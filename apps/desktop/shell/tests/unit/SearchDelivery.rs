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
    wakes.send(SearchDelivery::Shutdown).unwrap();
    thread.join().unwrap();
}

#[test]
fn stale_sessions_cannot_publish_or_acknowledge() {
    let session = SearchSession::new(2, tauri::ipc::Channel::new(|_| Ok(())));
    assert!(session.authorize(1).is_err());
    assert!(session.authorize(2).is_ok());
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
        view: nanika_protocol::View::Detail {
            detail: nanika_protocol::DetailView {
                title: None,
                content: nanika_protocol::DetailContent::Text {
                    value: "content".to_owned(),
                },
                metadata: Vec::new(),
                actions: Vec::new(),
            },
        },
    });
    assert!(session.begin_refresh(2).is_err());
    assert!(!session.navigation.busy);
}

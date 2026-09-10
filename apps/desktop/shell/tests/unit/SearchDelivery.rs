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

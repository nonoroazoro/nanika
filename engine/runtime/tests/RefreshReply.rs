use super::RefreshReply;
use nanika_protocol::Message;
use std::sync::{Arc, mpsc};

#[test]
fn only_the_matching_identity_completes_a_refresh_and_releases_admission() {
    let reply = Arc::new(RefreshReply::default());
    let (sender, receiver) = mpsc::channel();
    let next = Arc::clone(&reply);
    assert!(reply.register(
        "scan".to_owned(),
        7,
        Box::new(move |result| {
            sender.send(result).unwrap();
            // Completion runs outside the lock so the next request can register immediately.
            assert!(next.register("next".to_owned(), 8, Box::new(|_| {})));
        })
    ));
    for (id, generation) in [("other", 7), ("scan", 6)] {
        assert!(!reply.dispatch(&Ok(Some(Message::Refreshed {
            request_id: id.to_owned(),
            generation
        }))));
        assert!(receiver.try_recv().is_err());
    }
    assert!(!reply.dispatch(&Ok(Some(Message::CandidatesChanged))));
    assert!(reply.dispatch(&Ok(Some(Message::Refreshed {
        request_id: "scan".to_owned(),
        generation: 7
    }))));
    assert!(receiver.recv().unwrap().is_ok());
}

#[test]
fn eof_fails_pending_and_future_requests() {
    let reply = RefreshReply::default();
    let (sender, receiver) = mpsc::channel();
    reply.register(
        "scan".to_owned(),
        1,
        Box::new(move |result| sender.send(result).unwrap()),
    );
    assert!(!reply.dispatch(&Ok(None)));
    assert!(receiver.recv().unwrap().is_err());
    let (sender, receiver) = mpsc::channel();
    assert!(!reply.register(
        "later".to_owned(),
        2,
        Box::new(move |result| sender.send(result).unwrap())
    ));
    assert!(receiver.recv().unwrap().is_err());
}

#[test]
fn refresh_failure_does_not_consume_another_operations_error() {
    let reply = RefreshReply::default();
    let (sender, receiver) = mpsc::channel();
    reply.register(
        "scan".to_owned(),
        1,
        Box::new(move |result| sender.send(result).unwrap()),
    );
    assert!(!reply.dispatch(&Ok(Some(Message::Error {
        request_id: Some("query".to_owned()),
        code: "query_failed".to_owned(),
        message: "query error".to_owned()
    }))));
    assert!(receiver.try_recv().is_err());
    assert!(reply.dispatch(&Ok(Some(Message::Error {
        request_id: Some("scan".to_owned()),
        code: "scan_failed".to_owned(),
        message: "scan error".to_owned()
    }))));
    assert!(
        receiver
            .recv()
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("scan error")
    );
}

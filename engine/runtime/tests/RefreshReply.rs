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
        None,
        Box::new(move |result| {
            sender.send(result).unwrap();
            // Completion runs outside the lock so the next request can register immediately.
            assert!(next.register("next".to_owned(), 8, None, Box::new(|_| {})));
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
        None,
        Box::new(move |result| sender.send(result).unwrap())
    );
    assert!(!reply.dispatch(&Ok(None)));
    assert!(receiver.recv().unwrap().is_err());
    let (sender, receiver) = mpsc::channel();
    assert!(!reply.register(
        "later".to_owned(),
        2,
        None,
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
        None,
        Box::new(move |result| sender.send(result).unwrap())
    );
    assert!(!reply.dispatch(&Ok(Some(Message::Error {
        request_id: Some("query".to_owned()),
        error: nanika_protocol::ProtocolError::new(
            "query_failed".to_owned(),
            "query error".to_owned()
        )
    }))));
    assert!(receiver.try_recv().is_err());
    assert!(reply.dispatch(&Ok(Some(Message::Error {
        request_id: Some("scan".to_owned()),
        error: nanika_protocol::ProtocolError::new(
            "scan_failed".to_owned(),
            "scan error".to_owned()
        )
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

#[test]
fn global_failure_completes_refresh_without_consuming_the_shared_error() {
    for localized in [true, false] {
        let reply = RefreshReply::default();
        let (send, receive) = mpsc::channel();
        let resources = nanika_localization::ResourceSet::new(
            nanika_localization::SupportedLocale::English,
            [(
                nanika_localization::SupportedLocale::English,
                vec![("en.ftl".into(), "failed = Could not refresh".into())]
            )]
            .into(),
            false
        )
        .unwrap();
        reply.register(
            "scan".into(),
            1,
            localized.then(|| Arc::new(resources)),
            Box::new(move |result| send.send(result).unwrap())
        );
        assert!(!reply.dispatch(&Ok(Some(Message::Error {
            request_id: None,
            error: nanika_protocol::ProtocolError::new(
                "scan_failed",
                nanika_protocol::message("failed", [])
            )
        }))));
        let failure = receive.recv().unwrap().unwrap_err();
        if localized {
            let crate::SupervisorError::Reported(error) = failure else {
                panic!("typed failure");
            };
            assert!(
                error
                    .messages
                    .values()
                    .all(|text| text == "Could not refresh")
            );
        } else {
            assert!(matches!(
                failure,
                crate::SupervisorError::UnexpectedMessage(_)
            ));
        }
        assert!(reply.register("next".into(), 2, None, Box::new(|_| {})));
    }
}

use std::sync::{Arc, Mutex, mpsc};

use nanika_protocol::{Message, OperationProgress, message};

use crate::ConfigurationReply;

#[test]
fn invalid_progress_preserves_the_accepted_task_and_its_terminal_result() {
    let reply = ConfigurationReply::default();
    let resources = nanika_localization::ResourceSet::new(
        nanika_localization::SupportedLocale::English,
        [(
            nanika_localization::SupportedLocale::English,
            vec![("en.ftl".into(), "scan = Scanning".into())]
        )]
        .into(),
        false
    )
    .unwrap();
    let updates = Arc::new(Mutex::new(Vec::new()));
    let observed = updates.clone();
    let (completed, completion) = mpsc::channel();
    assert!(reply.register(
        "task".into(),
        Arc::new(move |value| observed.lock().unwrap().push(value)),
        Some(Arc::new(resources)),
        Box::new(move |result| completed.send(result).unwrap())
    ));
    for (id, label, count) in [
        ("task", "scan", 1),
        ("foreign", "scan", 2),
        ("task", "missing", 2)
    ] {
        assert!(reply.dispatch(&Ok(Some(Message::ConfigurationProgress {
            request_id: id.into(),
            progress: OperationProgress {
                label: message(label, []),
                completed: count,
                total: Some(3)
            }
        }))));
    }
    assert_eq!(
        updates.lock().unwrap().as_slice(),
        &[OperationProgress {
            label: message("scan", []),
            completed: 1,
            total: Some(3)
        }]
    );
    assert!(completion.try_recv().is_err());
    assert!(!reply.register(
        "replacement".into(),
        Arc::new(|_| panic!("not admitted")),
        None,
        Box::new(|result| assert!(result.is_err()))
    ));
    assert!(reply.dispatch(&Ok(Some(Message::ConfigurationApplied {
        request_id: "task".into()
    }))));
    completion.recv().unwrap().unwrap();
    assert!(reply.dispatch(&Ok(Some(Message::ConfigurationProgress {
        request_id: "task".into(),
        progress: OperationProgress {
            label: message("scan", []),
            completed: 3,
            total: Some(3)
        }
    }))));
    assert_eq!(updates.lock().unwrap().len(), 1);
}

#[test]
fn correlated_failure_uses_resources_retained_by_the_accepted_configuration() {
    let reply = ConfigurationReply::default();
    let resources = Arc::new(
        nanika_localization::ResourceSet::new(
            nanika_localization::SupportedLocale::English,
            [(
                nanika_localization::SupportedLocale::English,
                vec![("en.ftl".into(), "failed = Could not save".into())]
            )]
            .into(),
            false
        )
        .unwrap()
    );
    let (send, receive) = mpsc::channel();
    assert!(reply.register(
        "save".into(),
        Arc::new(|_| {}),
        Some(resources.clone()),
        Box::new(move |result| send.send(result).unwrap())
    ));
    drop(resources);
    for (id, consumed) in [("other", false), ("save", true)] {
        assert_eq!(
            reply.dispatch(&Ok(Some(Message::Error {
                request_id: Some(id.into()),
                error: nanika_protocol::ProtocolError::new("save_failed", message("failed", []))
                    .with_detail("disk full")
            }))),
            consumed
        );
        if !consumed {
            assert!(receive.try_recv().is_err());
        }
    }
    let crate::SupervisorError::Reported(error) = receive.recv().unwrap().unwrap_err() else {
        panic!("typed failure");
    };
    assert_eq!(error.error.code, "save_failed");
    assert_eq!(error.error.detail.as_deref(), Some("disk full"));
    assert!(error.messages.values().all(|text| text == "Could not save"));
    assert!(reply.register("next".into(), Arc::new(|_| {}), None, Box::new(|_| {})));
}

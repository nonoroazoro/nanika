use crate::TextReadReply;
use nanika_protocol::{Message, TextChunk, TextRead};
use std::sync::mpsc;

#[test]
fn stale_reads_never_block_or_enter_the_navigation_receiver() {
    let replies = TextReadReply::default();
    let read = TextRead {
        document_id: "doc".into(),
        revision: "rev".into(),
        index: 0
    };
    let (send, receive) = mpsc::channel();
    let first = send.clone();
    replies.register(
        "text-read-1".into(),
        1,
        read.clone(),
        Box::new(move |result| {
            first.send(result).unwrap();
        })
    );
    replies.register(
        "text-read-2".into(),
        2,
        read,
        Box::new(move |result| {
            send.send(result).unwrap();
        })
    );
    assert!(receive.recv().unwrap().is_err());
    let response = |id: &str, generation| {
        Ok(Some(Message::TextReadResult {
            request_id: id.into(),
            generation,
            chunk: TextChunk {
                document_id: "doc".into(),
                revision: "rev".into(),
                index: 0,
                value: "text".into()
            }
        }))
    };
    assert!(replies.dispatch(&response("text-read-1", 1)));
    assert!(receive.try_recv().is_err());
    assert!(replies.dispatch(&response("text-read-2", 2)));
    assert_eq!(receive.recv().unwrap().unwrap().value, "text");
    assert!(replies.dispatch(&response("text-read-2", 2)));
    assert!(!replies.dispatch(&Ok(Some(Message::ViewsChanged {
        text: Default::default()
    }))));
}

#[test]
fn rejects_response_revision_size_and_generation_mismatch() {
    for (revision, generation, value) in [
        ("old", 1, "text".to_owned()),
        ("rev", 2, "text".to_owned()),
        (
            "rev",
            1,
            "x".repeat(nanika_protocol::DETAIL_TEXT_BATCH_CHARS + 1)
        )
    ] {
        let replies = TextReadReply::default();
        let (send, receive) = mpsc::channel();
        replies.register(
            "text-read-1".into(),
            1,
            TextRead {
                document_id: "doc".into(),
                revision: "rev".into(),
                index: 0
            },
            Box::new(move |result| {
                send.send(result).unwrap();
            })
        );
        assert!(replies.dispatch(&Ok(Some(Message::TextReadResult {
            request_id: "text-read-1".into(),
            generation,
            chunk: TextChunk {
                document_id: "doc".into(),
                revision: revision.into(),
                index: 0,
                value
            }
        }))));
        assert!(receive.recv().unwrap().is_err());
    }
}

#[test]
fn terminal_transport_failure_rejects_future_registration() {
    let replies = TextReadReply::default();
    assert!(!replies.dispatch(&Ok(None)));
    let (send, receive) = mpsc::channel();
    assert!(!replies.register(
        "text-read-closed".into(),
        1,
        TextRead {
            document_id: "doc".into(),
            revision: "1".into(),
            index: 0
        },
        Box::new(move |result| {
            send.send(result).unwrap();
        })
    ));
    assert!(receive.recv().unwrap().is_err());
}

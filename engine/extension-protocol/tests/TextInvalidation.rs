use nanika_protocol::TextInvalidation;

#[test]
fn coalesced_revocations_preserve_ids_and_bound_memory() {
    let mut pending = TextInvalidation::default();
    pending.merge(TextInvalidation {
        documents: Some(vec!["a".into(), "b".into()])
    });
    pending.merge(TextInvalidation {
        documents: Some(vec!["a".into(), "c".into()])
    });
    assert_eq!(pending.documents.as_ref().unwrap(), &["a", "b", "c"]);
    pending.merge(TextInvalidation {
        documents: Some((0..129).map(|index| index.to_string()).collect())
    });
    assert!(pending.documents.is_none());
    pending.merge(TextInvalidation::default());
    assert!(pending.documents.is_none());
}

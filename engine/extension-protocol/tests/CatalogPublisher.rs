use nanika_protocol::{Action, Candidate, CandidateKind, CatalogPublisher};

fn entry(id: usize, title: &str) -> Candidate {
    Candidate {
        kind: CandidateKind::Action,
        entry_id: id.to_string(),
        title: title.into(),
        subtitle: None,
        action_id: "open".into(),
        actions: vec![Action::primary("open", "Open")],
        aliases: Vec::new(),
        icon: None,
    }
}

#[test]
fn fifty_thousand_entries_transfer_without_a_catalog_limit() {
    let mut publisher = CatalogPublisher::default();
    publisher.update((0..50_000).map(|id| entry(id, "Application")), []);
    let mut ids = std::collections::HashSet::new();
    let mut index = 0;
    loop {
        let batch = publisher.read().unwrap();
        assert!(batch.replace);
        assert_eq!(batch.index, index);
        assert!(batch.entries.len() <= 256);
        for entry in batch.entries {
            assert!(ids.insert(entry.entry_id));
        }
        index += 1;
        if batch.complete {
            assert!(publisher.read().is_err());
            assert!(!publisher.acknowledge(batch.transaction).unwrap());
            break;
        }
        assert!(publisher.acknowledge(batch.transaction).is_err());
    }
    assert_eq!(ids.len(), 50_000);
}

#[test]
fn updates_during_transfer_preserve_the_published_snapshot_and_next_delta() {
    let mut publisher = CatalogPublisher::default();
    publisher.update((0..600).map(|id| entry(id, "Before")), []);
    let first = publisher.read().unwrap();
    let mut entries = first.entries;
    publisher.update([entry(0, "After"), entry(601, "Added")], ["1".into()]);
    loop {
        let batch = publisher.read().unwrap();
        entries.extend(batch.entries);
        if batch.complete {
            assert!(publisher.acknowledge(batch.transaction).unwrap());
            break;
        }
    }
    assert_eq!(entries.len(), 600);
    assert!(entries.iter().all(|entry| entry.title == "Before"));
    let delta = publisher.read().unwrap();
    assert!(!delta.replace && delta.complete);
    assert_eq!(delta.entries.len(), 2);
    assert_eq!(delta.removed, ["1"]);
    assert!(publisher.acknowledge(delta.transaction + 1).is_err());
    assert!(!publisher.acknowledge(delta.transaction).unwrap());
    assert!(!publisher.update([entry(0, "After")], []));
}

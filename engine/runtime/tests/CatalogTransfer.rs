use crate::CatalogTransfer;
use nanika_protocol::{Action, Candidate, CandidateKind, CatalogBatch};

fn batch(transaction: u64, index: u64, complete: bool, id: &str) -> CatalogBatch {
    CatalogBatch {
        transaction,
        index,
        complete,
        replace: true,
        removed: Vec::new(),
        entries: vec![Candidate {
            kind: CandidateKind::Action,
            entry_id: id.into(),
            title: id.into(),
            subtitle: None,
            action_id: "open".into(),
            actions: vec![Action::primary("open", "Open")],
            aliases: Vec::new(),
            icon: None
        }]
    }
}

#[test]
fn partial_catalogs_cannot_commit_or_cross_transaction_boundaries() {
    let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
    let handle = owner.handle();
    let source = handle.register_extension("test", 1).unwrap();
    source.register_static_catalog(Vec::new()).unwrap();
    let mut transfer = CatalogTransfer::default();
    assert!(!transfer.accept(batch(1, 0, false, "a")).unwrap());
    assert!(transfer.commit(&source, &[], None).is_err());
    assert!(transfer.accept(batch(2, 1, true, "wrong")).is_err());
    assert!(transfer.accept(batch(1, 2, true, "gap")).is_err());
    assert!(transfer.accept(batch(1, 1, true, "b")).unwrap());
    assert_eq!(transfer.commit(&source, &[], None).unwrap(), Some(1));
    let generation = handle.begin_query("").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if let Some(snapshot) = handle
            .latest_snapshot()
            .filter(|snapshot| snapshot.generation == generation)
        {
            assert_eq!(snapshot.results.len(), 2);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(transfer.accept(batch(1, 0, true, "replay")).is_err());
    owner.shutdown();
}

#[test]
fn staging_budgets_reject_duplicates_and_empty_batches_without_replacing_published_data() {
    for byte_budget in [true, false] {
        let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
        let handle = owner.handle();
        let source = handle.register_extension("test", 1).unwrap();
        source.register_static_catalog(Vec::new()).unwrap();
        let mut transfer = CatalogTransfer::default();
        transfer.accept(batch(1, 0, true, "published")).unwrap();
        transfer.commit(&source, &[], None).unwrap();

        let mut first = batch(2, 0, false, "duplicate");
        first.removed = vec!["published".into()];
        if !byte_budget {
            first.entries.clear();
            first.removed.clear();
        }
        let bytes = serde_json::to_vec(&first).unwrap().len();
        transfer._budget = crate::CatalogBudget::new(
            if byte_budget { bytes * 2 } else { usize::MAX },
            if byte_budget { usize::MAX } else { 2 }
        );
        assert!(!transfer.accept(first.clone()).unwrap());
        first.index = 1;
        assert!(!transfer.accept(first.clone()).unwrap());
        first.index = 2;
        let error = transfer.accept(first.clone()).unwrap_err().to_string();
        assert!(error.contains(if byte_budget {
            "byte budget"
        } else {
            "batch budget"
        }));
        assert_eq!(transfer._entries.capacity(), 0);
        assert_eq!(transfer._removed.capacity(), 0);
        assert!(transfer.commit(&source, &[], None).is_err());
        first.complete = true;
        assert!(transfer.accept(first).is_err());

        let generation = handle.begin_query("").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(snapshot) = handle
                .latest_snapshot()
                .filter(|s| s.generation == generation)
            {
                assert_eq!(snapshot.results.len(), 1);
                assert_eq!(snapshot.results[0].candidate.entry_id(), "published");
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        owner.shutdown();
    }
}

#[test]
fn invalid_localization_keeps_the_published_catalog_and_cannot_be_retried_as_empty() {
    for replace in [true, false] {
        let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
        let handle = owner.handle();
        let source = handle.register_extension("test", 1).unwrap();
        source.register_static_catalog(Vec::new()).unwrap();
        let mut transfer = CatalogTransfer::default();
        transfer.accept(batch(1, 0, true, "published")).unwrap();
        transfer.commit(&source, &[], None).unwrap();
        let mut invalid = batch(2, 0, true, "invalid");
        invalid.replace = replace;
        invalid.removed.push("published".into());
        invalid.entries[0].title = nanika_protocol::message("missing", []);
        transfer.accept(invalid).unwrap();
        assert!(
            transfer
                .commit(&source, &[], None)
                .unwrap_err()
                .to_string()
                .contains("localization resources")
        );
        assert!(transfer.commit(&source, &[], None).is_err());
        let generation = handle.begin_query("").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(snapshot) = handle
                .latest_snapshot()
                .filter(|s| s.generation == generation)
            {
                assert_eq!(snapshot.results.len(), 1);
                assert_eq!(snapshot.results[0].candidate.entry_id(), "published");
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        owner.shutdown();
    }
}

#[test]
fn large_catalogs_fit_the_default_transaction_budget_and_release_it_after_commit() {
    let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
    let source = owner.handle().register_extension("test", 1).unwrap();
    source.register_static_catalog(Vec::new()).unwrap();
    let mut publisher = nanika_protocol::CatalogPublisher::default();
    publisher.update(
        (0..50_000).map(|id| batch(1, 0, true, &id.to_string()).entries.remove(0)),
        []
    );
    let mut transfer = CatalogTransfer::default();
    while !transfer.accept(publisher.read().unwrap()).unwrap() {}
    assert_eq!(transfer._entries.len(), 50_000);
    let transaction = transfer.commit(&source, &[], None).unwrap().unwrap();
    publisher.acknowledge(transaction).unwrap();
    assert!(transfer._entries.is_empty());
    publisher.update(batch(1, 0, true, "new").entries, []);
    assert!(transfer.accept(publisher.read().unwrap()).unwrap());
    assert!(transfer.commit(&source, &[], None).unwrap().is_some());
    owner.shutdown();
}

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
            icon: None,
        }],
    }
}

#[test]
fn partial_catalogs_cannot_commit_or_cross_transaction_boundaries() {
    let owner = nanika_search::SearchOwner::spawn(Default::default()).unwrap();
    let handle = owner.handle();
    handle.register_static_catalog("test", Vec::new()).unwrap();
    let mut transfer = CatalogTransfer::default();
    assert!(!transfer.accept("test", batch(1, 0, false, "a")).unwrap());
    assert!(transfer.commit(&handle, "test", Vec::new()).is_err());
    assert!(transfer.accept("test", batch(2, 1, true, "wrong")).is_err());
    assert!(transfer.accept("test", batch(1, 2, true, "gap")).is_err());
    assert!(transfer.accept("test", batch(1, 1, true, "b")).unwrap());
    assert_eq!(transfer.commit(&handle, "test", Vec::new()).unwrap(), 1);
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
    assert!(
        transfer
            .accept("test", batch(1, 0, true, "replay"))
            .is_err()
    );
    owner.shutdown();
}

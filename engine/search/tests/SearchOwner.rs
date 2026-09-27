use std::time::{Duration, Instant};

use nanika_search::{
    Candidate, CandidateKind, MAX_QUERY_CHARS, SearchOwner, SearchQueueError, UsageMap,
};

#[test]
fn owner_drops_stale_extension_snapshots() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let handle = owner.handle();
    let source_test_extension = handle.register_extension("test.extension", 1).unwrap();

    let stale = handle.begin_query("old").expect("query should enqueue");
    let current = handle.begin_query("tool").expect("query should enqueue");
    source_test_extension
        .publish_extension_snapshot(
            stale,
            vec![Candidate::new(
                CandidateKind::Action,
                "test.extension",
                "stale",
                "Old",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
            true,
        )
        .expect("stale snapshot should enqueue");
    source_test_extension
        .publish_extension_snapshot(
            current,
            vec![Candidate::new(
                CandidateKind::Action,
                "test.extension",
                "current",
                "Tool",
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                Vec::new(),
            )],
            true,
        )
        .expect("current snapshot should enqueue");

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = handle.latest_snapshot()
            && snapshot.generation == current
            && snapshot.results.len() == 1
        {
            assert_eq!(snapshot.results[0].candidate.entry_id(), "current");
            break;
        }
        assert!(Instant::now() < deadline, "current snapshot should arrive");
        std::thread::yield_now();
    }
    owner.shutdown();
}

#[test]
fn static_catalog_publishes_while_dynamic_contributors_are_pending() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source_static_extension = handle.register_extension("static.extension", 1).unwrap();
    let source_slow_extension = handle.register_extension("slow.extension", 2).unwrap();

    source_static_extension
        .register_static_catalog(vec![_review_candidate("static")])
        .unwrap();
    let generation = handle
        .begin_query_with_expected_extensions(
            "tool",
            ["static.extension".into(), "slow.extension".into()],
        )
        .unwrap();
    let initial = _wait_snapshot(&handle, |snapshot| {
        snapshot.generation == generation && snapshot.results.len() == 1
    });
    assert_eq!(initial.pending_extensions, ["slow.extension"]);
    assert_eq!(
        initial.results[0].candidate.extension_id(),
        "static.extension"
    );
    source_slow_extension
        .publish_extension_snapshot(generation, Vec::new(), true)
        .unwrap();
    let complete = _wait_snapshot(&handle, |snapshot| {
        snapshot.generation == generation && snapshot.pending_extensions.is_empty()
    });
    assert_eq!(complete.results.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&initial.results, &complete.results));
    let next = handle.begin_query("tool").unwrap();
    assert_eq!(
        _wait_snapshot(&handle, |snapshot| snapshot.generation == next)
            .results
            .len(),
        1
    );
    owner.shutdown();
}

#[test]
fn owner_rejects_oversized_queries_before_enqueueing() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let error = owner
        .handle()
        .begin_query("x".repeat(MAX_QUERY_CHARS + 1))
        .expect_err("oversized query should fail");
    assert_eq!(error, SearchQueueError::QueryTooLong);
    owner.shutdown();
}

#[test]
fn owner_coalesces_query_bursts_without_dropping_the_latest_query() {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("owner should start");
    let handle = owner.handle();
    let mut latest_generation = 0;
    for index in 0..1_000 {
        latest_generation = handle
            .begin_query(format!("query {index}"))
            .expect("latest query should never be dropped");
    }

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(snapshot) = handle.latest_snapshot()
            && snapshot.generation == latest_generation
            && snapshot.normalized_query == "query 999"
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "latest query should be published"
        );
        std::thread::yield_now();
    }
    owner.shutdown();
}

#[test]
fn owner_publishes_each_contributor_and_preserves_earlier_snapshots() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source_first_extension = handle.register_extension("first.extension", 1).unwrap();
    let source_second_extension = handle.register_extension("second.extension", 2).unwrap();

    let generation = handle
        .begin_query_with_expected_extensions(
            "tool",
            ["first.extension".into(), "second.extension".into()],
        )
        .unwrap();
    source_first_extension
        .publish_extension_snapshot(generation, vec![_review_candidate("first")], true)
        .unwrap();
    let partial = _wait_snapshot(&handle, |snapshot| {
        snapshot.generation == generation && snapshot.results.len() == 1
    });
    assert_eq!(partial.pending_extensions, ["second.extension"]);
    source_second_extension
        .publish_extension_snapshot(generation, vec![_review_candidate("second")], true)
        .unwrap();
    let complete = _wait_snapshot(&handle, |snapshot| {
        snapshot.generation == generation && snapshot.pending_extensions.is_empty()
    });
    assert_eq!(complete.results.len(), 2);
    assert_eq!(
        partial.results.len(),
        1,
        "published revisions remain immutable"
    );
    owner.shutdown();
}

#[test]
fn only_expected_identities_clear_pending_contributors() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source_unexpected_extension = handle
        .register_extension("unexpected.extension", 1)
        .unwrap();
    let source_first_extension = handle.register_extension("first.extension", 2).unwrap();
    let source_second_extension = handle.register_extension("second.extension", 3).unwrap();

    let generation = handle
        .begin_query_with_expected_extensions(
            "tool",
            ["first.extension".into(), "second.extension".into()],
        )
        .unwrap();
    source_unexpected_extension
        .publish_extension_snapshot(generation, Vec::new(), true)
        .unwrap();
    source_first_extension
        .publish_extension_snapshot(generation, Vec::new(), true)
        .unwrap();
    let partial = _wait_snapshot(&handle, |snapshot| {
        snapshot.generation == generation && snapshot.pending_extensions == ["second.extension"]
    });
    assert!(partial.results.is_empty());
    source_second_extension.retire().unwrap();
    _wait_snapshot(&handle, |snapshot| {
        snapshot.generation == generation && snapshot.pending_extensions.is_empty()
    });
    owner.shutdown();
}

#[test]
fn concurrent_query_admission_cannot_publish_an_older_generation_last() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let concurrent = handle.clone();
    let (entered, waiting) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        concurrent
            .begin_query_with_expected_extensions(
                "later",
                std::iter::from_fn(move || {
                    entered.send(()).unwrap();
                    released.recv().unwrap();
                    None
                }),
            )
            .unwrap()
    });
    waiting.recv_timeout(Duration::from_secs(2)).unwrap();
    let first = handle.begin_query("first").unwrap();
    release.send(()).unwrap();
    let last = thread.join().unwrap();
    assert!(last > first);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !handle
        .latest_snapshot()
        .is_some_and(|snapshot| snapshot.generation == last)
    {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    owner.shutdown();
}

#[test]
fn patches_change_only_their_entries_and_cannot_revive_a_retired_catalog() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source_test = handle.register_extension("test", 1).unwrap();
    let source_barrier = handle.register_extension("barrier", 2).unwrap();

    let generation = handle
        .begin_query_with_expected_extensions("", ["test".to_owned()])
        .unwrap();
    let entry = |id: &str, title: &str| {
        Candidate::new(
            CandidateKind::Action,
            "untrusted",
            id,
            title,
            "open",
            vec![nanika_protocol::Action::primary("open", "Open")],
            Vec::new(),
        )
    };
    source_test
        .publish_extension_snapshot(
            generation,
            vec![
                entry("a", "Original"),
                entry("b", "Keep"),
                entry("c", "Remove"),
            ],
            true,
        )
        .unwrap();
    _wait_snapshot(&handle, |snapshot| snapshot.results.len() == 3);
    source_test
        .publish_extension_delta(
            generation,
            vec![entry("a", "Updated")],
            vec!["c".to_owned()],
            true,
        )
        .unwrap();
    let snapshot = _wait_snapshot(&handle, |snapshot| snapshot.results.len() == 2);
    assert_eq!(snapshot.results.len(), 2);
    assert!(
        snapshot
            .results
            .iter()
            .any(|entry| entry.candidate.title() == "Updated")
    );
    assert!(
        snapshot
            .results
            .iter()
            .any(|entry| entry.candidate.title() == "Keep")
    );
    assert!(
        snapshot
            .results
            .iter()
            .all(|entry| entry.candidate.extension_id() == "test")
    );
    source_test.retire().unwrap();

    source_test
        .publish_extension_delta(generation, vec![entry("a", "Retired")], Vec::new(), true)
        .unwrap();
    source_barrier.retire().unwrap();
    assert!(handle.latest_snapshot().unwrap().results.is_empty());
    owner.shutdown();
}

#[test]
fn catalog_commits_are_independent_of_queries_and_share_unchanged_metadata() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source_catalog = handle.register_extension("catalog", 1).unwrap();

    let entry = |id: &str, title: &str| {
        Candidate::new(
            CandidateKind::Action,
            "catalog",
            id,
            title,
            "open",
            vec![nanika_protocol::Action::primary("open", "Open")],
            Vec::new(),
        )
    };
    source_catalog
        .register_static_catalog(vec![entry("keep", "Keep")])
        .unwrap();
    handle.begin_query("cancelled").unwrap();
    let generation = handle.begin_query("").unwrap();
    source_catalog
        .commit_catalog(false, vec![entry("new", "New")], Vec::new())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let before = loop {
        if let Some(snapshot) = handle
            .latest_snapshot()
            .filter(|s| s.generation == generation && s.results.len() == 2)
        {
            break snapshot;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    source_catalog
        .commit_catalog(false, vec![entry("new", "Updated")], Vec::new())
        .unwrap();
    let after = loop {
        if let Some(snapshot) = handle.latest_snapshot().filter(|s| {
            s.results
                .iter()
                .any(|row| row.candidate.title() == "Updated")
        }) {
            break snapshot;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    let old = &before
        .results
        .iter()
        .find(|row| row.candidate.entry_id() == "keep")
        .unwrap()
        .candidate;
    let new = &after
        .results
        .iter()
        .find(|row| row.candidate.entry_id() == "keep")
        .unwrap()
        .candidate;
    assert!(std::ptr::eq(old.title().as_ptr(), new.title().as_ptr()));
    assert!(
        before
            .results
            .iter()
            .any(|row| row.candidate.title() == "New")
    );
    source_catalog.retire().unwrap();
    assert!(
        source_catalog
            .commit_catalog(false, vec![entry("late", "Late")], Vec::new())
            .is_err()
    );
    owner.shutdown();
}

fn _review_candidate(id: &str) -> Candidate {
    Candidate::new(
        CandidateKind::Action,
        "owner",
        id,
        format!("{id} tool"),
        "open",
        vec![nanika_protocol::Action::primary("open", "Open")],
        Vec::new(),
    )
}

fn _wait_snapshot(
    handle: &nanika_search::SearchHandle,
    matches: impl Fn(&nanika_search::SearchSnapshot) -> bool,
) -> std::sync::Arc<nanika_search::SearchSnapshot> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(snapshot) = handle.latest_snapshot()
            && matches(&snapshot)
        {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "expected snapshot did not arrive"
        );
        std::thread::yield_now();
    }
}

#[test]
fn partial_snapshots_and_empty_terminal_deltas_preserve_request_completion() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source_stream = handle.register_extension("stream", 1).unwrap();
    let source_barrier = handle.register_extension("barrier", 2).unwrap();

    let generation = handle
        .begin_query_with_expected_extensions("tool", ["stream".into()])
        .unwrap();
    source_stream
        .publish_extension_snapshot(generation, vec![_review_candidate("first")], false)
        .unwrap();
    let partial = _wait_snapshot(&handle, |s| {
        s.generation == generation && s.results.len() == 1
    });
    assert_eq!(partial.pending_extensions, ["stream"]);
    source_stream
        .publish_extension_delta(generation, Vec::new(), Vec::new(), true)
        .unwrap();
    let completed = _wait_snapshot(&handle, |s| {
        s.generation == generation && s.pending_extensions.is_empty()
    });
    assert_eq!(completed.results.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&partial.results, &completed.results));
    source_stream
        .set_extension_query_pending(generation, true)
        .unwrap();
    let refresh = _wait_snapshot(&handle, |s| s.pending_extensions == ["stream"]);
    assert_eq!(refresh.results.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&completed.results, &refresh.results));
    assert_eq!(completed.authority(), refresh.authority());
    source_stream
        .set_extension_query_pending(generation, false)
        .unwrap();
    let cancelled = _wait_snapshot(&handle, |s| s.pending_extensions.is_empty());
    assert_eq!(
        cancelled.results.len(),
        1,
        "cancellation retains already published results"
    );
    assert!(std::sync::Arc::ptr_eq(&refresh.results, &cancelled.results));
    let next = handle.begin_query("").unwrap();
    source_stream
        .set_extension_query_pending(generation, true)
        .unwrap();
    source_barrier.retire().unwrap();
    let current = _wait_snapshot(&handle, |s| s.generation == next);
    assert!(
        current.pending_extensions.is_empty(),
        "stale admission cannot affect the new query"
    );
    owner.shutdown();
}

#[test]
fn old_instance_publications_and_retirement_cannot_affect_its_replacement() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let old = handle.register_extension("test.extension", 1).unwrap();
    old.register_static_catalog(Vec::new()).unwrap();
    old.retire().unwrap();
    let new = handle.register_extension("test.extension", 2).unwrap();
    new.register_static_catalog(vec![Candidate::new(
        CandidateKind::Action,
        "test.extension",
        "new",
        "New",
        "open",
        Vec::new(),
        Vec::new(),
    )])
    .unwrap();
    let generation = handle.begin_query("").unwrap();
    let before = _wait_snapshot(&handle, |s| s.results.len() == 1);
    old.publish_extension_snapshot(
        generation,
        vec![Candidate::new(
            CandidateKind::Action,
            "test.extension",
            "old",
            "Old",
            "open",
            Vec::new(),
            Vec::new(),
        )],
        true,
    )
    .unwrap();
    old.set_extension_query_pending(generation, true).unwrap();
    old.retire().unwrap();
    assert_eq!(
        old.commit_catalog(true, Vec::new(), Vec::new()),
        Err(SearchQueueError::Retired)
    );
    let after = handle.latest_snapshot().unwrap();
    assert_eq!(before.authority(), after.authority());
    assert_eq!(after.results[0].candidate.entry_id(), "new");
    assert_eq!(after.instances["test.extension"], 2);
    assert!(after.pending_extensions.is_empty());
    owner.shutdown();
}

#[test]
fn query_admission_revokes_old_authority_before_the_owner_can_publish() {
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let (entered, ready) = std::sync::mpsc::sync_channel(1);
    let (release, resumed) = std::sync::mpsc::sync_channel(1);
    let resumed = std::sync::Mutex::new(resumed);
    let first = std::sync::atomic::AtomicBool::new(true);
    handle.set_notifier(std::sync::Arc::new(move || {
        if first.swap(false, std::sync::atomic::Ordering::SeqCst) {
            entered.send(()).unwrap();
            resumed.lock().unwrap().recv().unwrap();
        }
    }));
    handle.begin_query("old").unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let visible = handle.latest_snapshot().unwrap();
    assert!(handle.is_current(visible.authority()));
    handle.begin_query("new").unwrap();
    let rejected = !handle.is_current(visible.authority());
    let not_yet_published = handle.latest_snapshot().unwrap().authority() == visible.authority();
    release.send(()).unwrap();
    owner.shutdown();
    assert!(not_yet_published);
    assert!(rejected);
}

#[test]
fn off_query_changes_and_icons_preserve_authority_but_publish_new_presentation() {
    use std::sync::{Arc, mpsc};
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source = handle.register_extension("test.extension", 1).unwrap();
    let candidate = |id, title| {
        Candidate::new(
            CandidateKind::Action,
            "test.extension",
            id,
            title,
            "open",
            vec![nanika_protocol::Action::primary("open", "Open")],
            vec![],
        )
    };
    source
        .register_static_catalog(vec![
            candidate("visible", "needle"),
            candidate("hidden", "zzzz"),
        ])
        .unwrap();
    let generation = handle.begin_query("needle").unwrap();
    let before = _wait_snapshot(&handle, |s| {
        s.generation == generation && s.results.len() == 1
    });
    // Registration is a queue barrier; the first publication's notifier has finished.
    let _barrier = handle.register_extension("test.barrier", 2).unwrap();
    let (sent, received) = mpsc::channel();
    handle.set_notifier(Arc::new(move || {
        let _ = sent.send(());
    }));
    source
        .commit_catalog(false, vec![candidate("hidden", "yyyy")], vec![])
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let unchanged = handle.latest_snapshot().unwrap();
    assert_eq!(before.authority(), unchanged.authority());
    assert!(Arc::ptr_eq(&before.results, &unchanged.results));
    source
        .commit_catalog(
            false,
            vec![
                candidate("visible", "needle").with_icon(Some(nanika_protocol::IconSource::Empty)),
            ],
            vec![],
        )
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let decorated = handle.latest_snapshot().unwrap();
    assert_eq!(before.authority(), decorated.authority());
    assert!(!Arc::ptr_eq(&before.results, &decorated.results));
    assert_eq!(
        decorated.results[0].candidate.icon(),
        Some(&nanika_protocol::IconSource::Empty)
    );
    assert!(handle.is_current(before.authority()));
    owner.shutdown();
}

#[test]
fn reviewed_metadata_and_order_revoke_authority_while_score_only_changes_do_not() {
    use std::sync::{Arc, mpsc};
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let handle = owner.handle();
    let source = handle.register_extension("test.extension", 1).unwrap();
    let candidate = |id: &str, title: &str, action| {
        Candidate::new(
            CandidateKind::Action,
            "test.extension",
            id,
            title,
            "open",
            vec![action],
            vec![],
        )
    };
    let action = nanika_protocol::Action::primary("open", "Open");
    source
        .register_static_catalog(vec![
            candidate("a", "Needle", action.clone()),
            candidate("b", "Needle", action.clone()),
        ])
        .unwrap();
    let generation = handle.begin_query("needle").unwrap();
    let before = _wait_snapshot(&handle, |s| {
        s.generation == generation && s.results.len() == 2
    });
    let _barrier = handle.register_extension("test.barrier", 2).unwrap();
    let (sent, received) = mpsc::channel();
    handle.set_notifier(Arc::new(move || {
        let _ = sent.send(());
    }));
    let key = nanika_search::UsageKey::new("test.extension", "b", "open", "needle");
    handle
        .apply_persisted_execution(key.clone(), u64::MAX)
        .unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    let reordered = handle.latest_snapshot().unwrap();
    assert_eq!(reordered.results[0].candidate.entry_id(), "b");
    assert_ne!(before.authority(), reordered.authority());
    handle.apply_persisted_execution(key, u64::MAX).unwrap();
    received.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        reordered.authority(),
        handle.latest_snapshot().unwrap().authority()
    );
    let mut disabled = action.clone();
    disabled.enabled = false;
    let mut confirmed = action.clone();
    confirmed.allow_default_execution = false;
    confirmed.style = nanika_protocol::ActionStyle::Destructive;
    confirmed.confirmation_title = Some("Confirm".into());
    for changed in [
        candidate("b", "Needle renamed", action.clone()),
        candidate("b", "Needle renamed", action.clone()).with_subtitle(Some(
            nanika_protocol::CandidateSubtitle::Description("Changed path".into()),
        )),
        candidate("b", "Needle renamed", disabled),
        candidate("b", "Needle renamed", confirmed),
    ] {
        let reviewed = handle.latest_snapshot().unwrap();
        source.commit_catalog(false, vec![changed], vec![]).unwrap();
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!handle.is_current(reviewed.authority()));
    }
    owner.shutdown();
}

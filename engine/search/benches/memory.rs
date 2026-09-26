//! Counts live requested heap bytes, excluding allocator overhead and native/WebView memory.
#![allow(unsafe_code)]
use nanika_search::{Candidate, CandidateKind, SearchEngine, UsageMap};
#[path = "support/TrackingAllocator.rs"]
mod tracking_allocator;
use std::sync::atomic::Ordering;
use tracking_allocator::{LIVE, PEAK, TrackingAllocator};

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

fn main() {
    println!("entries,index_bytes,retained_ranking_bytes,peak_query_bytes");
    for count in [100, 1_000, 5_000, 10_000, 50_000] {
        let baseline = LIVE.load(Ordering::Relaxed);
        let entries = (0..count)
            .map(|index| {
                Candidate::new(
                    CandidateKind::Action,
                    "benchmark",
                    format!("entry-{index}"),
                    format!("Application {index}"),
                    "open",
                    vec![nanika_protocol::Action::primary("open", "Open")],
                    Vec::new(),
                )
            })
            .collect::<Vec<_>>();
        let index_bytes = LIVE.load(Ordering::Relaxed) - baseline;
        let mut engine = SearchEngine::new();
        let usage = UsageMap::new();
        let before = LIVE.load(Ordering::Relaxed);
        PEAK.store(before, Ordering::Relaxed);
        let snapshot = engine.query("", &entries, &usage, 0);
        assert_eq!(snapshot.results.len(), count);
        let retained = LIVE.load(Ordering::Relaxed) - before;
        let peak = PEAK.load(Ordering::Relaxed) - before;
        println!("{count},{index_bytes},{retained},{peak}");
        std::hint::black_box((&entries, &snapshot));
    }
}

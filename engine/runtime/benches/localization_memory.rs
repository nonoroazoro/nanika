//! Requested Rust heap bytes and allocations only; excludes allocator overhead and native/WebView memory.
#![allow(unsafe_code)]
use std::hint::black_box;
use std::sync::atomic::Ordering;

use nanika_search::{SearchEngine, UsageMap};

#[path = "support/localization.rs"]
mod support;
#[path = "../../search/benches/support/TrackingAllocator.rs"]
mod tracking_allocator;
use tracking_allocator::{ALLOCATIONS, LIVE, PEAK, TrackingAllocator};

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

fn main() {
    let english = support::resources(false);
    let bilingual = support::resources(true);
    black_box(support::candidates(100, Some(&bilingual)));
    let empty_usage = UsageMap::new();
    let usage = support::usage();
    println!(
        "Requested heap counters exclude shared resources and process-global romanization tables."
    );
    for count in support::SIZES {
        for (mode, resources) in [
            ("literal", None),
            ("english", Some(&english)),
            ("bilingual", Some(&bilingual))
        ] {
            let before = LIVE.load(Ordering::Relaxed);
            let allocations = ALLOCATIONS.load(Ordering::Relaxed);
            PEAK.store(before, Ordering::Relaxed);
            let entries = support::candidates(count, resources);
            let retained = LIVE.load(Ordering::Relaxed) - before;
            let peak = PEAK.load(Ordering::Relaxed) - before;
            let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations;
            println!(
                "{}",
                serde_json::json!({"kind": "admission", "mode": mode, "entries": count, "retained_bytes": retained, "peak_bytes": peak, "allocations": allocations})
            );
            let mut engine = SearchEngine::new();
            for (name, query) in support::QUERIES {
                let history = if name == "usage" {
                    &usage
                } else {
                    &empty_usage
                };
                black_box(engine.query(query, &entries, history, 1_000));
                let start = LIVE.load(Ordering::Relaxed);
                let allocations = ALLOCATIONS.load(Ordering::Relaxed);
                PEAK.store(start, Ordering::Relaxed);
                let snapshot = engine.query(query, &entries, history, 1_000);
                let retained = LIVE.load(Ordering::Relaxed) - start;
                let peak = PEAK.load(Ordering::Relaxed) - start;
                let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations;
                println!(
                    "{}",
                    serde_json::json!({"kind": "query", "mode": mode, "entries": count, "query": name, "matches": snapshot.results.len(), "retained_bytes": retained, "peak_bytes": peak, "allocations": allocations})
                );
            }
            drop(engine);
            drop(entries);
            let residual = LIVE.load(Ordering::Relaxed).saturating_sub(before);
            println!(
                "{}",
                serde_json::json!({"kind": "released", "mode": mode, "entries": count, "residual_bytes": residual})
            );
        }
    }
}

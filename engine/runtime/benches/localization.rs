//! Individual operation latency without an instrumented allocator or a browser/WebView.
use std::hint::black_box;
use std::time::{Duration, Instant};

use nanika_search::{SearchEngine, UsageMap};

#[path = "support/admission_phases.rs"]
mod admission_phases;

#[path = "support/localization.rs"]
mod support;

fn main() {
    let phases = std::env::args().any(|argument| argument == "--admission-phases");
    let admission_only = std::env::args().any(|argument| argument == "--admission-only");
    let english = support::resources(false);
    let bilingual = support::resources(true);
    // Prime process-global romanization tables before comparing admission workloads.
    black_box(support::candidates(100, Some(&bilingual)));
    let empty_usage = UsageMap::new();
    let usage = support::usage();
    for count in support::SIZES {
        for (mode, resources) in [
            ("literal", None),
            ("english", Some(&english)),
            ("bilingual", Some(&bilingual))
        ] {
            let mut admission = Vec::with_capacity(31);
            for _ in 0..31 {
                let start = Instant::now();
                let entries = support::candidates(count, resources);
                admission.push(start.elapsed());
                black_box(entries);
            }
            _report("admission", mode, count, "", count, &mut admission);
            if phases {
                admission_phases::measure(mode, count, resources);
            }
            if admission_only || phases {
                continue;
            }
            let entries = support::candidates(count, resources);
            let mut engine = SearchEngine::new();
            for (name, query) in support::QUERIES {
                let history = if name == "usage" {
                    &usage
                } else {
                    &empty_usage
                };
                for _ in 0..32 {
                    black_box(engine.query(query, &entries, history, 1_000));
                }
                let mut samples = Vec::with_capacity(1_001);
                let mut matches = 0;
                for _ in 0..1_001 {
                    let start = Instant::now();
                    let snapshot = engine.query(black_box(query), &entries, history, 1_000);
                    samples.push(start.elapsed());
                    matches = snapshot.results.len();
                    black_box(snapshot);
                }
                _report("query", mode, count, name, matches, &mut samples);
            }
        }
    }
}

fn _report(
    kind: &str,
    mode: &str,
    entries: usize,
    query: &str,
    matches: usize,
    samples: &mut [Duration]
) {
    samples.sort_unstable();
    let percentile = |percent: usize| {
        samples[(samples.len() * percent).div_ceil(100) - 1].as_secs_f64() * 1_000_000.0
    };
    println!(
        "{}",
        serde_json::json!({
            "kind": kind, "mode": mode, "entries": entries, "query": query, "matches": matches,
            "samples": samples.len(), "p50_us": percentile(50), "p95_us": percentile(95), "p99_us": percentile(99)
        })
    );
}

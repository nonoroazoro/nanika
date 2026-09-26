use criterion::{Criterion, criterion_group, criterion_main};
use nanika_search::{Candidate, CandidateKind, SearchEngine, UsageKey, UsageMap, UsageStat};
use std::hint::black_box;

fn ranking_benchmark(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("catalog");
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_secs(1));
    for count in [100, 1_000, 5_000, 10_000, 50_000] {
        let entries = make_candidates(count);
        let mut engine = SearchEngine::new();
        let usage = UsageMap::new();
        for (name, query) in [("empty", ""), ("fuzzy", "app42")] {
            group.bench_function(format!("{name}/{count}"), |bencher| {
                bencher.iter(|| black_box(engine.query(black_box(query), &entries, &usage, 0)));
            });
        }
    }
    group.finish();
    let candidates = make_candidates(1_000);
    let large_candidates = make_candidates(20_000);
    criterion.bench_function("rank_1000_candidates", |bencher| {
        let mut engine = SearchEngine::new();
        let usage = UsageMap::new();
        bencher
            .iter(|| black_box(engine.query(black_box("application 42"), &candidates, &usage, 0)));
    });
    criterion.bench_function("rank_1000_candidates_with_usage", |bencher| {
        let mut engine = SearchEngine::new();
        let mut usage = UsageMap::new();
        for index in 0..100 {
            usage.insert(
                UsageKey::new(
                    "benchmark",
                    &format!("entry-{index}"),
                    "launch",
                    "application 42",
                ),
                UsageStat {
                    execution_count: 10,
                    last_executed_at: 1_000,
                },
            );
        }
        bencher.iter(|| {
            black_box(engine.query(black_box("application 42"), &candidates, &usage, 1_000))
        });
    });
    criterion.bench_function("rank_20000_candidates", |bencher| {
        let mut engine = SearchEngine::new();
        let usage = UsageMap::new();
        bencher.iter(|| {
            black_box(engine.query(black_box("application 42"), &large_candidates, &usage, 0))
        });
    });
}

fn make_candidates(count: usize) -> Vec<Candidate> {
    (0..count)
        .map(|index| {
            Candidate::new(
                CandidateKind::Action,
                "benchmark",
                format!("entry-{index}"),
                format!("Application {index}"),
                "launch",
                vec![nanika_protocol::Action::primary("launch", "Open")],
                Vec::new(),
            )
        })
        .collect()
}

criterion_group!(benches, ranking_benchmark);
criterion_main!(benches);

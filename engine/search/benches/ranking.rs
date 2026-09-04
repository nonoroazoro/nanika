use criterion::{Criterion, criterion_group, criterion_main};
use nanika_search::{Candidate, SearchEngine, UsageKey, UsageMap, UsageStat};
use std::hint::black_box;

fn ranking_benchmark(criterion: &mut Criterion) {
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
                "benchmark",
                format!("entry-{index}"),
                format!("Application {index}"),
                "launch",
                Vec::new(),
            )
        })
        .collect()
}

criterion_group!(benches, ranking_benchmark);
criterion_main!(benches);

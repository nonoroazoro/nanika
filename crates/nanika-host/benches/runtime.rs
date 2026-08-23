use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use nanika_config::ConfigStore;
use nanika_host::prepare_visible_results;
use nanika_search::{Candidate, SearchOwner, SearchSnapshot, UsageKey, UsageMap, UsageStat};
use nanika_storage::SearchStorageWorker;

fn runtime_benchmarks(criterion: &mut Criterion) {
    query_delivery_benchmark(criterion);
    runtime_foundation_startup_benchmark(criterion);
    render_preparation_benchmark(criterion);
}

fn query_delivery_benchmark(criterion: &mut Criterion) {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let candidates = make_candidates(1_000);

    criterion.bench_function("search_query_delivery_1000", |bencher| {
        bencher.iter_batched(
            || candidates.clone(),
            |candidates| {
                let generation = search
                    .begin_query(black_box("application 42"))
                    .expect("query should be accepted");
                search
                    .publish_extension_snapshot("benchmark", generation, candidates)
                    .expect("snapshot should be accepted");
                wait_for_generation(&search, generation)
            },
            BatchSize::PerIteration,
        );
    });

    owner.shutdown();
}

fn runtime_foundation_startup_benchmark(criterion: &mut Criterion) {
    criterion.bench_function("runtime_foundation_startup", |bencher| {
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let root = temporary_root();
                let machine_root = root.join("machine");
                let config_root = root.join("config");
                let start = Instant::now();
                let store = ConfigStore::open(&machine_root, &config_root)
                    .expect("configuration should open");
                let (storage, state) = SearchStorageWorker::spawn(
                    root.join("data").join("databases").join("nanika.db"),
                    100,
                )
                .expect("storage owner should start");
                let usage = state
                    .usage
                    .into_iter()
                    .map(|stored| {
                        (
                            UsageKey::new(
                                &stored.extension_id,
                                &stored.entry_id,
                                &stored.action_id,
                                &stored.query_context,
                            ),
                            UsageStat {
                                execution_count: stored.execution_count,
                                last_executed_at: stored.last_executed_at,
                            },
                        )
                    })
                    .collect();
                let search = SearchOwner::spawn(usage).expect("search owner should start");
                black_box(store);
                elapsed += start.elapsed();
                search.shutdown();
                storage.shutdown();
                std::fs::remove_dir_all(root).expect("benchmark directory should be removable");
            }
            elapsed
        });
    });
}

fn render_preparation_benchmark(criterion: &mut Criterion) {
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner should start");
    let search = owner.handle();
    let generation = search
        .begin_query("application")
        .expect("query should be accepted");
    search
        .publish_extension_snapshot("benchmark", generation, make_candidates(100))
        .expect("snapshot should be accepted");
    let snapshot = wait_for_generation(&search, generation);

    criterion.bench_function("render_preparation_100", |bencher| {
        bencher.iter(|| {
            let prepared = prepare_visible_results(
                black_box(snapshot.as_ref()),
                black_box(generation),
                black_box(3),
            );
            let mut title_bytes = 0;
            for (_, result, selected) in prepared {
                title_bytes += black_box(result.candidate.title().len());
                title_bytes += usize::from(black_box(selected));
            }
            black_box(title_bytes)
        });
    });

    owner.shutdown();
}

fn wait_for_generation(
    search: &nanika_search::SearchHandle,
    generation: u64,
) -> std::sync::Arc<SearchSnapshot> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(snapshot) = search.latest_snapshot()
            && snapshot.generation == generation
            && !snapshot.results.is_empty()
        {
            return snapshot;
        }
        assert!(Instant::now() < deadline, "search owner timed out");
        std::thread::yield_now();
    }
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

fn temporary_root() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!("nanika-benchmark-{}-{unique}", std::process::id()))
}

criterion_group!(benches, runtime_benchmarks);
criterion_main!(benches);

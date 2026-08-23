use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{Criterion, SamplingMode, criterion_group, criterion_main};
use nanika_host::ExtensionProcess;

fn extension_activation_benchmark(criterion: &mut Criterion) {
    let fixture = fixture_path();
    let mut group = criterion.benchmark_group("extension_activation");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("fixture_process", |bencher| {
        bencher.iter(|| {
            let mut extension =
                ExtensionProcess::spawn(black_box(&fixture)).expect("fixture should spawn");
            extension
                .initialize("benchmark-initialize")
                .expect("fixture should initialize");
            extension
                .shutdown("benchmark-shutdown")
                .expect("fixture should stop");
        });
    });
    group.finish();
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nanika-extension-fixture"))
}

criterion_group!(benches, extension_activation_benchmark);
criterion_main!(benches);

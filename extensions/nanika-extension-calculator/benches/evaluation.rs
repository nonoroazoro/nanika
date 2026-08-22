use criterion::{Criterion, criterion_group, criterion_main};
use nanika_extension_calculator::CalculatorEngine;
use std::hint::black_box;

fn evaluation(criterion: &mut Criterion) {
    let engine = CalculatorEngine::new();
    criterion.bench_function("calculator_preview_arithmetic", |bencher| {
        bencher.iter(|| engine.evaluate(black_box("(12345 * 6789) / 3")));
    });
    criterion.bench_function("calculator_preview_units", |bencher| {
        bencher.iter(|| engine.evaluate(black_box("15 km / 30 min to km/h")));
    });
}

criterion_group!(benches, evaluation);
criterion_main!(benches);

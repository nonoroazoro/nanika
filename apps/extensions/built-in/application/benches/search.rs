use criterion::{Criterion, criterion_group, criterion_main};
use nanika_search::{Candidate, CandidateKind, SearchEngine, UsageMap};
use std::hint::black_box;

fn search(criterion: &mut Criterion) {
    let names = [
        "同步",
        "音乐",
        "重庆",
        "银行",
        "朝阳",
        "人行道",
        "中国银行",
        "网易云音乐",
        "长安",
        "同步 Sync",
    ];
    let entries = (0..2_000)
        .map(|index| {
            Candidate::new(
                CandidateKind::Action,
                "application",
                format!("app.{index}"),
                format!("{} {index:04}", names[index % names.len()]),
                "open",
                vec![nanika_protocol::Action::primary("open", "Open")],
                vec!["music".into()],
            )
        })
        .collect::<Vec<_>>();
    let mut engine = SearchEngine::new();
    let usage = UsageMap::new();
    for (name, query) in [
        ("original", "音乐"),
        ("romanized", "yinyue"),
        ("mixed", "音yue"),
        ("cross_alias", "音乐 music"),
    ] {
        assert!(!engine.query(query, &entries, &usage, 0).results.is_empty());
        criterion.bench_function(&format!("application_query_2000_{name}"), |bencher| {
            bencher.iter(|| black_box(engine.query(black_box(query), &entries, &usage, 0)));
        });
    }
}
criterion_group!(benches, search);
criterion_main!(benches);

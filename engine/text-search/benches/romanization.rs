use std::hint::black_box;
use std::mem::size_of;

use criterion::{Criterion, criterion_group, criterion_main};
use nanika_text_search::{romanized_query, romanized_readings};
use pinyin::ToPinyin;

fn catalog() -> Vec<String> {
    const NAMES: &[&str] = &[
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
    (0..2_000)
        .map(|index| format!("{} {index:04}", NAMES[index % NAMES.len()]))
        .collect()
}

fn romanization(criterion: &mut Criterion) {
    let names = catalog();
    criterion.bench_function("text_search_character_first_2000", |bencher| {
        bencher.iter(|| {
            names
                .iter()
                .map(|name| {
                    name.chars()
                        .filter_map(|character| character.to_pinyin())
                        .map(|reading| reading.plain())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        });
    });
    criterion.bench_function("text_search_phrase_aliases_2000", |bencher| {
        bencher.iter(|| {
            names
                .iter()
                .map(|name| romanized_readings(black_box(name)))
                .collect::<Vec<_>>()
        });
    });
    let aliases = names
        .iter()
        .map(|name| romanized_readings(name))
        .collect::<Vec<_>>();
    let retained_capacity = aliases.capacity()
        * size_of::<Vec<nanika_text_search::RomanizedReading>>()
        + aliases
            .iter()
            .map(|readings| {
                readings.capacity() * size_of::<nanika_text_search::RomanizedReading>()
                    + readings
                        .iter()
                        .map(|reading| reading.full.capacity() + reading.initials.capacity())
                        .sum::<usize>()
            })
            .sum::<usize>();
    eprintln!(
        "text_search_prepared_alias_capacity_2000={retained_capacity} bytes (excluding allocator metadata and source names)"
    );
    criterion.bench_function("text_search_mixed_query_2000", |bencher| {
        bencher.iter(|| {
            let query = romanized_query(black_box("同bu"));
            aliases
                .iter()
                .filter(|readings| {
                    readings
                        .iter()
                        .any(|reading| query.iter().any(|query| reading.full.contains(&query.full)))
                })
                .count()
        });
    });
}

criterion_group!(benches, romanization);
criterion_main!(benches);

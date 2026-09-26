use criterion::{Criterion, criterion_group, criterion_main};
use nanika_extension_clipboard::{ClipboardConfig, ClipboardDatabase, ClipboardEntry};
use nanika_protocol::ClipboardContent;

fn persistence(criterion: &mut Criterion) {
    let root =
        std::env::temp_dir().join(format!("nanika-clipboard-benchmark-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("benchmark root");
    let database = ClipboardDatabase::open(root.join("clipboard.db")).expect("database");
    for index in 0..500 {
        database
            .upsert(&entry(index, index as u64))
            .expect("seed entry");
    }
    let mut captured_at = 501_u64;
    criterion.bench_function("clipboard_upsert_deduplicated_text", |bencher| {
        let mut item = entry(42, captured_at);
        bencher.iter(|| {
            item.captured_at = captured_at;
            captured_at = captured_at.saturating_add(1);
            database.upsert(&item).expect("entry should persist")
        });
    });
    criterion.bench_function("clipboard_load_500", |bencher| {
        bencher.iter(|| database.load().expect("history should load"));
    });
    for count in [500, 5000] {
        let history = ClipboardDatabase::open(root.join(format!("capture-{count}.db"))).unwrap();
        for index in 0..count {
            let mut item = entry(index, index as u64);
            item.content = ClipboardContent::Text {
                value: "x".repeat(4096),
            };
            item.byte_size = 4096;
            history.upsert(&item).unwrap();
        }
        let mut entries = history.load().unwrap();
        let config = ClipboardConfig {
            max_entries: None,
            max_age_days: None,
        };
        criterion.bench_function(&format!("clipboard_capture_publish_{count}"), |bencher| {
            let mut item = entry(42, count as u64);
            bencher.iter(|| {
                item.captured_at += 1;
                let change = history
                    .upsert_with_retention(&item, item.captured_at, &config)
                    .unwrap();
                change.apply(&mut entries, Some(item.clone()));
                std::hint::black_box(&entries);
            });
        });
    }
    drop(database);
    let _ = std::fs::remove_dir_all(root);
}

fn entry(index: usize, captured_at: u64) -> ClipboardEntry {
    let value = format!("Clipboard benchmark payload {index}");
    ClipboardEntry {
        entry_id: format!("clipboard.{index}"),
        title: value.clone(),
        byte_size: value.len() as u64,
        content: ClipboardContent::Text { value },
        captured_at,
    }
}

criterion_group!(benches, persistence);
criterion_main!(benches);

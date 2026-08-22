use criterion::{Criterion, criterion_group, criterion_main};
use nanika_extension_clipboard::{ClipboardDatabase, ClipboardEntry};
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
    drop(database);
    let _ = std::fs::remove_dir_all(root);
}

fn entry(index: usize, captured_at: u64) -> ClipboardEntry {
    let value = format!("Clipboard benchmark payload {index}");
    ClipboardEntry {
        entry_id: format!("clipboard.{index}"),
        content_hash: index.to_string(),
        title: value.clone(),
        byte_size: value.len() as u64,
        content: ClipboardContent::Text { value },
        captured_at,
        pinned: false,
    }
}

criterion_group!(benches, persistence);
criterion_main!(benches);

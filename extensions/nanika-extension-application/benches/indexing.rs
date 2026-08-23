use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use nanika_extension_application::{
    ApplicationConfig, ApplicationDatabase, ApplicationIndex, IconCache, select_candidates,
};
use nanika_protocol::Message;

const ENTRY_COUNT: usize = 500;

fn indexing(criterion: &mut Criterion) {
    let root = test_root();
    let applications = root.join("applications");
    std::fs::create_dir_all(&applications).expect("application root should exist");
    create_executables(&applications);
    let database_path = root.join("application.db");
    let icon_root = root.join("icons");
    let database = ApplicationDatabase::open(&database_path).expect("database should open");
    let mut index = ApplicationIndex::new(database, IconCache::new(&icon_root));
    let config = ApplicationConfig {
        format_version: 1,
        roots: vec![applications],
        exclusions: ApplicationConfig::standard_roots().expect("standard roots"),
    };
    let cancellation = AtomicU64::new(0);
    let (_, entries) = index
        .scan(&config, 1, &cancellation)
        .expect("warm scan should complete");

    criterion.bench_function("application_index_cold_validation_500", |bencher| {
        bencher.iter_batched(
            || {
                let database =
                    ApplicationDatabase::open(&database_path).expect("database should reopen");
                ApplicationIndex::new(database, IconCache::new(&icon_root))
            },
            |mut cold_index| {
                cold_index
                    .scan(&config, 2, &cancellation)
                    .expect("cold validation scan should complete")
            },
            BatchSize::SmallInput,
        );
    });

    let mut generation = 3_u64;
    criterion.bench_function("application_index_500", |bencher| {
        bencher.iter(|| {
            let result = index
                .scan(&config, generation, &cancellation)
                .expect("scan should complete");
            generation = generation.saturating_add(1);
            result
        });
    });

    let candidates = entries
        .iter()
        .map(nanika_extension_application::ApplicationEntry::candidate)
        .collect::<Vec<_>>();
    let snapshot = Message::Snapshot {
        request_id: "benchmark".to_owned(),
        generation: 1,
        complete: true,
        entries: candidates,
    };
    criterion.bench_function("application_snapshot_json_500", |bencher| {
        bencher.iter(|| serde_json::to_vec(&snapshot).expect("snapshot should serialize"));
    });
    let large_entries = (0..10_000)
        .map(|index| {
            let mut entry = entries[index % entries.len()].clone();
            entry.entry_id = format!("app.{index}");
            entry.display_name = format!("Application {index:04}");
            entry.normalized_name = entry.display_name.to_lowercase();
            entry.normalized_tokens = entry.normalized_name.clone();
            entry
        })
        .collect::<Vec<_>>();
    criterion.bench_function("application_preselection_10000", |bencher| {
        bencher.iter(|| select_candidates(&large_entries, "application 9999", 5_000));
    });
    drop(index);
    std::fs::remove_dir_all(root).expect("benchmark directory should be removable");
}

fn create_executables(root: &Path) {
    let mut executable = vec![0_u8; 68];
    executable[..2].copy_from_slice(b"MZ");
    executable[60..64].copy_from_slice(&64_u32.to_le_bytes());
    executable[64..68].copy_from_slice(b"PE\0\0");
    for index in 0..ENTRY_COUNT {
        let target = root.join(format!("Application {index:04}.exe"));
        std::fs::write(target, &executable).expect("benchmark executable should exist");
    }
}

fn test_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-benchmark-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("benchmark root should exist");
    root
}

criterion_group!(benches, indexing);
criterion_main!(benches);

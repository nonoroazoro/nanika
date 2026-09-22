use std::hint::black_box;
use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{Criterion, criterion_group, criterion_main};
use nanika_extension_application::{ApplicationDatabase, ApplicationEntry, select_candidates};
use nanika_protocol::Message;
use nanika_search::{Candidate, CandidateKind, SearchEngine, UsageMap};
use nanika_text_search::romanized_readings;

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

fn catalog() -> Vec<ApplicationEntry> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("nanika-query-bench-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&root).expect("unique benchmark directory");
    let path = root.join("applications.db");
    let database = ApplicationDatabase::open(&path).expect("benchmark database");
    let mut connection = rusqlite::Connection::open(&path).expect("benchmark writer");
    let transaction = connection.transaction().expect("benchmark transaction");
    {
        let mut statement = transaction
            .prepare("INSERT INTO app_entries (entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, arguments_json, icon_key) VALUES (?1, ?2, ?3, ?4, '', 'executable', ?5, ?6, '')")
            .expect("benchmark insert");
        for index in 0..2_000 {
            let name = format!("{} {index:04}", NAMES[index % NAMES.len()]);
            statement
                .execute(rusqlite::params![
                    format!("app.{index}"),
                    format!("source.{index}"),
                    name,
                    name,
                    format!("Application-{index}.exe"),
                    r#"{"kind":"structured","values":[]}"#,
                ])
                .expect("benchmark row");
        }
    }
    transaction.commit().expect("benchmark data commit");
    let mut entries = database.load_entries().expect("benchmark entries");
    drop(connection);
    drop(database);
    std::fs::remove_dir_all(&root).expect("remove owned benchmark directory");
    for entry in &mut entries {
        entry.search_readings = romanized_readings(&entry.display_name);
    }
    entries
        .iter_mut()
        .find(|entry| entry.display_name.starts_with("音乐 "))
        .expect("music fixture")
        .normalized_tokens = "music".to_owned();
    entries
}

fn complete_query(entries: &[ApplicationEntry], query: &str, engine: &mut SearchEngine) -> usize {
    let snapshot = Message::Snapshot {
        request_id: "benchmark".to_owned(),
        generation: 1,
        complete: true,
        entries: select_candidates(entries, query),
    };
    let serialized = serde_json::to_vec(&snapshot).expect("serialize application snapshot");
    let received: Message =
        serde_json::from_slice(&serialized).expect("decode application snapshot");
    let Message::Snapshot { entries, .. } = received else {
        unreachable!("decoded snapshot has the same variant")
    };
    let candidates = entries
        .into_iter()
        .map(|entry| {
            Candidate::new(
                CandidateKind::Action,
                "application",
                entry.entry_id,
                entry.title,
                entry.action_id,
                entry.aliases,
            )
        })
        .collect::<Vec<_>>();
    let results = engine.query(query, &candidates, &UsageMap::new(), 0);
    let count = results.results.len();
    black_box(results);
    count
}

fn search(criterion: &mut Criterion) {
    let prepared = catalog();
    let mut original = prepared.clone();
    for entry in &mut original {
        entry.search_readings.clear();
    }
    let mut engine = SearchEngine::new();
    assert!(
        complete_query(&prepared, "音乐 music", &mut engine) > 0,
        "cross-name benchmark must exercise a real hit"
    );
    criterion.bench_function("application_query_2000_original", |bencher| {
        bencher.iter(|| complete_query(black_box(&original), black_box("音乐"), &mut engine));
    });
    criterion.bench_function("application_query_2000_romanized", |bencher| {
        bencher.iter(|| complete_query(black_box(&prepared), black_box("yinyue"), &mut engine));
    });
    criterion.bench_function("application_query_2000_mixed", |bencher| {
        bencher.iter(|| complete_query(black_box(&prepared), black_box("音yue"), &mut engine));
    });
    criterion.bench_function("application_query_2000_cross_alias", |bencher| {
        bencher.iter(|| complete_query(black_box(&prepared), black_box("音乐 music"), &mut engine));
    });
}

criterion_group!(benches, search);
criterion_main!(benches);

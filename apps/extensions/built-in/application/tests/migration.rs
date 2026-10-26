use super::*;

#[test]
fn legacy_cache_upgrade_preserves_records_and_accepts_translated_updates() {
    let path =
        std::env::temp_dir().join(format!("nanika-name-migration-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    connection.execute("INSERT INTO app_sources VALUES ('root','app.id','source','NetEaseMusic','music','macos-bundle','/Applications/Music.app','{\"kind\":\"structured\",\"values\":[]}', 'icon', NULL, 7)", []).unwrap();
    drop(connection);
    let mut database = crate::ApplicationDatabase::open(&path).unwrap();
    let mut entries = database.load_entries().unwrap();
    assert_eq!(entries.len(), 1);
    let original = entries[0].clone();
    assert_eq!(original.entry_id, "app.id");
    assert_eq!(original.source_key, "source");
    assert_eq!(original.target_path, "/Applications/Music.app");
    assert_eq!(original.normalized_tokens, "music");
    assert_eq!(original.icon_key, "icon");
    assert_eq!(original.priority, 7);
    assert!(original.localized_names.is_empty());
    entries[0].localized_names = [
        ("en".into(), "NetEaseMusic".into()),
        ("zh-Hans".into(), "网易云音乐".into())
    ]
    .into();
    database.commit_source("root", &entries, &[]).unwrap();
    drop(database);
    let reopened = crate::ApplicationDatabase::open(&path).unwrap();
    assert_eq!(reopened.load_entries().unwrap(), entries);
    assert_eq!(
        entries[0].launch_descriptor().unwrap(),
        original.launch_descriptor().unwrap()
    );
    drop(reopened);
    let connection = open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    drop(connection);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn unknown_cache_schema_is_rejected_without_deleting_records() {
    let path = std::env::temp_dir().join(format!(
        "nanika-name-unknown-schema-{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let connection = nanika_database::open(&path, LEGACY_SCHEMA).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE user_data (value TEXT); INSERT INTO user_data VALUES ('retain');"
        )
        .unwrap();
    drop(connection);
    assert!(open(&path).is_err());
    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT value FROM user_data", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "retain"
    );
    drop(connection);
    std::fs::remove_file(path).unwrap();
}

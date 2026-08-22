pub(crate) const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    "
CREATE TABLE scan_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    generation INTEGER NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    last_error TEXT
);
INSERT INTO scan_state (id, generation, status) VALUES (1, 0, 'idle');
CREATE TABLE app_entries (
    entry_id TEXT PRIMARY KEY,
    source_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    normalized_tokens TEXT NOT NULL,
    launch_kind TEXT NOT NULL,
    target_path TEXT NOT NULL,
    working_directory TEXT,
    arguments_json TEXT NOT NULL,
    bundle_id TEXT,
    icon_key TEXT NOT NULL,
    file_identity TEXT NOT NULL,
    last_seen_at INTEGER NOT NULL,
    stale INTEGER NOT NULL DEFAULT 0 CHECK (stale IN (0, 1))
);
CREATE INDEX app_entries_stale_name ON app_entries(stale, normalized_name);
CREATE INDEX app_entries_file_identity ON app_entries(file_identity);
",
)];

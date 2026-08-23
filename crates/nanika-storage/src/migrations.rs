pub(crate) const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    "
CREATE TABLE extensions (
    extension_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    installed_version TEXT,
    active_version TEXT,
    install_path TEXT,
    package_digest TEXT,
    state TEXT NOT NULL,
    health TEXT NOT NULL,
    last_error TEXT,
    updated_at INTEGER NOT NULL
);
CREATE TABLE input_history (
    id INTEGER PRIMARY KEY,
    normalized_query TEXT NOT NULL UNIQUE,
    display_query TEXT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 0,
    first_used_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL
);
CREATE TABLE usage_stats (
    extension_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    query_context TEXT NOT NULL,
    execution_count INTEGER NOT NULL DEFAULT 0,
    last_executed_at INTEGER NOT NULL,
    PRIMARY KEY (extension_id, entry_id, action_id, query_context),
    FOREIGN KEY (extension_id) REFERENCES extensions(extension_id) ON DELETE CASCADE
);
CREATE INDEX usage_stats_last_executed_at
ON usage_stats(last_executed_at DESC);
",
)];

pub(crate) const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    "
CREATE TABLE clipboard_entries (
    entry_id TEXT PRIMARY KEY,
    content_kind TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    title TEXT NOT NULL,
    text_payload TEXT,
    files_json TEXT,
    image_path TEXT,
    byte_size INTEGER NOT NULL,
    captured_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    UNIQUE(content_kind, content_hash)
);
CREATE INDEX clipboard_entries_retention
ON clipboard_entries(pinned DESC, captured_at DESC);
",
)];

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nanika_protocol::ClipboardContent;
use rusqlite::{Connection, params};

use crate::{ClipboardConfig, ClipboardEntry, EncodedClipboardContent};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS clipboard_entries (
    entry_id TEXT PRIMARY KEY CHECK (entry_id <> ''),
    content_kind TEXT NOT NULL,
    content_hash TEXT NOT NULL CHECK (content_hash <> ''),
    title TEXT NOT NULL CHECK (title <> ''),
    text_payload TEXT,
    files_json TEXT,
    image_path TEXT,
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    captured_at INTEGER NOT NULL CHECK (captured_at >= 0),
    UNIQUE(content_kind, content_hash),
    CHECK (
        (content_kind = 'text' AND text_payload IS NOT NULL AND files_json IS NULL AND image_path IS NULL)
        OR (content_kind = 'files' AND text_payload IS NULL AND files_json IS NOT NULL AND image_path IS NULL)
        OR (content_kind = 'image' AND text_payload IS NULL AND files_json IS NULL AND image_path IS NOT NULL)
    )
) STRICT;
CREATE INDEX IF NOT EXISTS clipboard_entries_ordering
ON clipboard_entries(captured_at DESC, entry_id);
PRAGMA user_version=1;
";

pub struct ClipboardDatabase {
    connection: Connection,
}

impl ClipboardDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|error| error.to_string())?;
        Ok(Self { connection })
    }

    pub fn upsert(&self, entry: &ClipboardEntry) -> Result<(), String> {
        write_entry(&self.connection, entry)
    }

    pub fn upsert_with_retention(
        &self,
        entry: &ClipboardEntry,
        now: u64,
        config: &ClipboardConfig,
    ) -> Result<HashSet<PathBuf>, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        write_entry(&transaction, entry)?;
        prune(&transaction, now, config)?;
        let retained = image_paths(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(retained)
    }

    pub fn apply_retention(
        &self,
        now: u64,
        config: &ClipboardConfig,
    ) -> Result<HashSet<PathBuf>, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        prune(&transaction, now, config)?;
        let retained = image_paths(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(retained)
    }

    pub fn load(&self) -> Result<Vec<ClipboardEntry>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT entry_id, content_kind, content_hash, title, text_payload, files_json,
                        image_path, byte_size, captured_at
                 FROM clipboard_entries
                 ORDER BY captured_at DESC, entry_id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        rows.into_iter()
            .map(
                |(entry_id, kind, content_hash, title, text, files, image, size, captured)| {
                    Ok(ClipboardEntry {
                        entry_id,
                        content_hash,
                        title,
                        content: decode_content(&kind, text, files, image)?,
                        byte_size: u64::try_from(size)
                            .map_err(|error| format!("invalid clipboard byte size: {error}"))?,
                        captured_at: u64::try_from(captured)
                            .map_err(|error| format!("invalid clipboard capture time: {error}"))?,
                    })
                },
            )
            .collect()
    }

    pub fn clear(&self, entry_ids: &[String]) -> Result<HashSet<PathBuf>, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        {
            let mut statement = transaction
                .prepare("DELETE FROM clipboard_entries WHERE entry_id = ?1")
                .map_err(|error| error.to_string())?;
            for entry_id in entry_ids {
                statement
                    .execute([entry_id])
                    .map_err(|error| error.to_string())?;
            }
        }
        let retained = image_paths(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(retained)
    }
}

fn write_entry(connection: &Connection, entry: &ClipboardEntry) -> Result<(), String> {
    let encoded = encode_content(&entry.content)?;
    connection
        .execute(
            "INSERT INTO clipboard_entries (
                    entry_id, content_kind, content_hash, title, text_payload, files_json,
                    image_path, byte_size, captured_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(content_kind, content_hash) DO UPDATE SET
                    title = excluded.title,
                    text_payload = excluded.text_payload,
                    files_json = excluded.files_json,
                    image_path = excluded.image_path,
                    byte_size = excluded.byte_size,
                    captured_at = excluded.captured_at",
            params![
                entry.entry_id,
                encoded.kind,
                entry.content_hash,
                entry.title,
                encoded.text,
                encoded.files,
                encoded.image,
                integer(entry.byte_size),
                integer(entry.captured_at),
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn prune(connection: &Connection, now: u64, config: &ClipboardConfig) -> Result<(), String> {
    if let Some(cutoff) = config.cutoff_millis(now) {
        connection
            .execute(
                "DELETE FROM clipboard_entries WHERE captured_at < ?1",
                [integer(cutoff)],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(maximum) = config.max_entries {
        connection
            .execute(
                "DELETE FROM clipboard_entries
             WHERE entry_id IN (
                 SELECT entry_id
                 FROM clipboard_entries
                 ORDER BY captured_at DESC, entry_id
                 LIMIT -1 OFFSET ?1
             )",
                [i64::from(maximum)],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn image_paths(connection: &Connection) -> Result<HashSet<PathBuf>, String> {
    let mut statement = connection
        .prepare(
            "SELECT image_path FROM clipboard_entries
             WHERE content_kind = 'image' AND image_path IS NOT NULL",
        )
        .map_err(|error| error.to_string())?;
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .map(|path| path.map(PathBuf::from).map_err(|error| error.to_string()))
        .collect()
}

fn encode_content(content: &ClipboardContent) -> Result<EncodedClipboardContent, String> {
    match content {
        ClipboardContent::Text { value } => Ok(EncodedClipboardContent {
            kind: "text",
            text: Some(value.clone()),
            files: None,
            image: None,
        }),
        ClipboardContent::Files { paths } => Ok(EncodedClipboardContent {
            kind: "files",
            text: None,
            files: Some(serde_json::to_string(paths).map_err(|error| error.to_string())?),
            image: None,
        }),
        ClipboardContent::PngFile { path } => Ok(EncodedClipboardContent {
            kind: "image",
            text: None,
            files: None,
            image: Some(path.clone()),
        }),
    }
}

fn decode_content(
    kind: &str,
    text: Option<String>,
    files: Option<String>,
    image: Option<String>,
) -> Result<ClipboardContent, String> {
    match kind {
        "text" => Ok(ClipboardContent::Text {
            value: text.ok_or_else(|| "clipboard text payload is missing".to_owned())?,
        }),
        "files" => Ok(ClipboardContent::Files {
            paths: serde_json::from_str(
                files
                    .as_deref()
                    .ok_or_else(|| "clipboard file payload is missing".to_owned())?,
            )
            .map_err(|error| error.to_string())?,
        }),
        "image" => Ok(ClipboardContent::PngFile {
            path: image.ok_or_else(|| "clipboard image payload is missing".to_owned())?,
        }),
        _ => Err(format!("unknown clipboard content kind: {kind}")),
    }
}

fn integer(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

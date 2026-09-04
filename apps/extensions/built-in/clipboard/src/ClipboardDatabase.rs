use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nanika_protocol::ClipboardContent;
use rusqlite::{Connection, params};

use crate::{ClipboardEntry, EncodedClipboardContent};

const RETENTION_SECONDS: u64 = 30 * 86_400;
const MAX_UNPINNED_ENTRIES: usize = 500;
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS clipboard_entries (
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
CREATE INDEX IF NOT EXISTS clipboard_entries_retention
ON clipboard_entries(pinned DESC, captured_at DESC);
";

/// Clipboard extension SQLite owner boundary.
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
        let encoded = encode_content(&entry.content)?;
        self.connection
            .execute(
                "INSERT INTO clipboard_entries (
                    entry_id, content_kind, content_hash, title, text_payload, files_json,
                    image_path, byte_size, captured_at, last_used_at, pinned
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10)
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
                    entry.pinned,
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn load(&self) -> Result<Vec<ClipboardEntry>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT entry_id, content_kind, content_hash, title, text_payload, files_json,
                        image_path, byte_size, captured_at, pinned
                 FROM clipboard_entries
                 ORDER BY pinned DESC, captured_at DESC, entry_id
                 LIMIT 5000",
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
                    row.get::<_, bool>(9)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        rows.into_iter()
            .map(
                |(
                    entry_id,
                    kind,
                    content_hash,
                    title,
                    text,
                    files,
                    image,
                    size,
                    captured,
                    pinned,
                )| {
                    Ok(ClipboardEntry {
                        entry_id,
                        content_hash,
                        title,
                        content: decode_content(&kind, text, files, image)?,
                        byte_size: u64::try_from(size).unwrap_or(0),
                        captured_at: u64::try_from(captured).unwrap_or(0),
                        pinned,
                    })
                },
            )
            .collect()
    }

    pub fn mark_used(&self, entry_id: &str, used_at: u64) -> Result<(), String> {
        self.connection
            .execute(
                "UPDATE clipboard_entries SET last_used_at = ?1 WHERE entry_id = ?2",
                params![integer(used_at), entry_id],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn prune(&self, now: u64) -> Result<(), String> {
        let cutoff = now.saturating_sub(RETENTION_SECONDS);
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM clipboard_entries WHERE pinned = 0 AND captured_at < ?1",
                [integer(cutoff)],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM clipboard_entries
                 WHERE pinned = 0 AND entry_id NOT IN (
                    SELECT entry_id FROM clipboard_entries
                    WHERE pinned = 0
                    ORDER BY captured_at DESC, entry_id
                    LIMIT ?1
                 )",
                [i64::try_from(MAX_UNPINNED_ENTRIES).unwrap_or(i64::MAX)],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn cleanup_payloads(&self, payload_root: &Path) -> Result<(), String> {
        std::fs::create_dir_all(payload_root).map_err(|error| error.to_string())?;
        let referenced = self.image_paths()?;
        for entry in std::fs::read_dir(payload_root).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if !path.is_file() || referenced.contains(&path) {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if is_managed_payload(&name) {
                std::fs::remove_file(path).map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    fn image_paths(&self) -> Result<HashSet<PathBuf>, String> {
        let mut statement = self
            .connection
            .prepare("SELECT image_path FROM clipboard_entries WHERE image_path IS NOT NULL")
            .map_err(|error| error.to_string())?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .map(|path| path.map(PathBuf::from).map_err(|error| error.to_string()))
            .collect()
    }
}

fn is_managed_payload(name: &str) -> bool {
    let Some((hash, suffix)) = name.split_once('.') else {
        return false;
    };
    let valid_hash = hash.len() == 64
        && hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
    valid_hash
        && (suffix == "png"
            || suffix.strip_prefix("tmp-").is_some_and(|pid| {
                !pid.is_empty() && pid.bytes().all(|byte| byte.is_ascii_digit())
            }))
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

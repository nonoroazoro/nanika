use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nanika_protocol::ClipboardContent;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{ClipboardChange, ClipboardConfig, ClipboardEntry, EncodedClipboardContent};

const SCHEMA: &str = "
CREATE TABLE clipboard_entries (
    entry_id TEXT PRIMARY KEY CHECK (entry_id <> ''),
    content_kind TEXT NOT NULL,
    title TEXT NOT NULL CHECK (title <> ''),
    text_payload TEXT,
    files_json TEXT,
    image_path TEXT,
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    captured_at INTEGER NOT NULL CHECK (captured_at >= 0),
    CHECK (
        (content_kind = 'text' AND text_payload IS NOT NULL AND files_json IS NULL AND image_path IS NULL)
        OR (content_kind = 'files' AND text_payload IS NULL AND files_json IS NOT NULL AND image_path IS NULL)
        OR (content_kind = 'image' AND text_payload IS NULL AND files_json IS NULL AND image_path IS NOT NULL)
    )
) STRICT;
CREATE INDEX clipboard_entries_ordering
ON clipboard_entries(captured_at DESC, entry_id);
CREATE INDEX clipboard_entries_images ON clipboard_entries(image_path)
WHERE content_kind = 'image' AND image_path IS NOT NULL;
PRAGMA user_version=1;
";

pub struct ClipboardDatabase {
    connection: Connection,
}

impl ClipboardDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let connection = nanika_database::open(path, SCHEMA).map_err(|error| error.to_string())?;
        Ok(Self { connection })
    }

    pub(crate) fn retained_images(&self) -> Result<HashSet<PathBuf>, String> {
        image_paths(&self.connection)
    }

    pub fn upsert(&self, entry: &ClipboardEntry) -> Result<(), String> {
        write_entry(&self.connection, entry)
    }

    pub fn upsert_with_retention(
        &self,
        entry: &ClipboardEntry,
        now: u64,
        config: &ClipboardConfig,
    ) -> Result<ClipboardChange, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        let mut affected = HashSet::new();
        let previous: Option<String> = transaction
            .query_row(
                "SELECT image_path FROM clipboard_entries WHERE entry_id = ?1",
                [&entry.entry_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();
        if let Some(path) = previous {
            affected.insert(PathBuf::from(path));
        }
        if let ClipboardContent::PngFile { path } = &entry.content {
            affected.insert(PathBuf::from(path));
        }
        write_entry(&transaction, entry)?;
        let removed = prune(&transaction, now, config, &mut affected)?;
        let image_ownership = _image_ownership(&transaction, affected)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(ClipboardChange {
            removed,
            image_ownership,
        })
    }

    pub fn apply_retention(
        &self,
        now: u64,
        config: &ClipboardConfig,
    ) -> Result<ClipboardChange, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        let mut affected = HashSet::new();
        let removed = prune(&transaction, now, config, &mut affected)?;
        let image_ownership = _image_ownership(&transaction, affected)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(ClipboardChange {
            removed,
            image_ownership,
        })
    }

    /// SQLite streams one payload at a time; only matching identities are retained.
    pub fn matching_ids(&self, query: &str, content_type: &str) -> Result<Vec<String>, String> {
        Ok(self
            .matching_entries(query, content_type)?
            .into_iter()
            .map(|entry| entry.entry_id)
            .collect())
    }

    pub(crate) fn matching_entries(
        &self,
        query: &str,
        content_type: &str,
    ) -> Result<Vec<crate::ClipboardQueryEntry>, String> {
        let kind = crate::query::content_kind(content_type)?;
        if query.is_empty() {
            let mut statement = self.connection.prepare("SELECT entry_id, captured_at FROM clipboard_entries WHERE (?1 IS NULL OR content_kind = ?1) ORDER BY captured_at DESC, entry_id").map_err(|e| e.to_string())?;
            return statement
                .query_map([kind], |row| {
                    Ok(crate::ClipboardQueryEntry {
                        entry_id: row.get(0)?,
                        captured_at: row.get(1)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string());
        }
        let mut statement = self.connection.prepare("SELECT entry_id, title, text_payload, files_json, content_kind, image_path, captured_at FROM clipboard_entries WHERE (?1 IS NULL OR content_kind = ?1) ORDER BY captured_at DESC, entry_id").map_err(|e| e.to_string())?;
        let mut rows = statement.query([kind]).map_err(|e| e.to_string())?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let title: String = row.get(1).map_err(|e| e.to_string())?;
            let content = decode_content(
                row.get_ref(4)
                    .and_then(|value| Ok(value.as_str()?))
                    .map_err(|e| e.to_string())?,
                row.get(2).map_err(|e| e.to_string())?,
                row.get(3).map_err(|e| e.to_string())?,
                row.get(5).map_err(|e| e.to_string())?,
            )?;
            if crate::query::matches(query, &title, &content) {
                ids.push(crate::ClipboardQueryEntry {
                    entry_id: row.get(0).map_err(|e| e.to_string())?,
                    captured_at: row.get(6).map_err(|e| e.to_string())?,
                });
            }
        }
        Ok(ids)
    }

    pub(crate) fn items(&self, ids: &[String]) -> Result<Vec<crate::ClipboardItem>, String> {
        let mut statement = self.connection.prepare("SELECT entry_id, title, content_kind, json_extract(files_json, '$[0]') FROM clipboard_entries WHERE entry_id = ?1").map_err(|e| e.to_string())?;
        ids.iter()
            .map(|id| {
                statement
                    .query_row([id], |row| {
                        Ok(crate::ClipboardItem {
                            entry_id: row.get(0)?,
                            title: row.get(1)?,
                            kind: row.get(2)?,
                            first_path: row.get(3)?,
                        })
                    })
                    .map_err(|e| e.to_string())
            })
            .collect()
    }

    pub fn entry(&self, id: &str) -> Result<ClipboardEntry, String> {
        let (entry_id, kind, title, text, files, image, size, captured) = self.connection.query_row(
            "SELECT entry_id, content_kind, title, text_payload, files_json, image_path, byte_size, captured_at FROM clipboard_entries WHERE entry_id = ?1", [id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,Option<String>>(3)?,row.get::<_,Option<String>>(4)?,row.get::<_,Option<String>>(5)?,row.get::<_,i64>(6)?,row.get::<_,i64>(7)?))
        ).map_err(|e| e.to_string())?;
        Ok(ClipboardEntry {
            entry_id,
            title,
            content: decode_content(&kind, text, files, image)?,
            byte_size: u64::try_from(size).map_err(|e| e.to_string())?,
            captured_at: u64::try_from(captured).map_err(|e| e.to_string())?,
        })
    }

    #[cfg(test)]
    pub fn load(&self) -> Result<Vec<ClipboardEntry>, String> {
        self.matching_ids("", "all")?
            .iter()
            .map(|id| self.entry(id))
            .collect()
    }

    pub fn clear(&self, entry_ids: &[String]) -> Result<ClipboardChange, String> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        let mut removed = HashSet::new();
        let mut affected = HashSet::new();
        {
            let mut statement = transaction
                .prepare("DELETE FROM clipboard_entries WHERE entry_id = ?1 RETURNING entry_id, image_path")
                .map_err(|error| error.to_string())?;
            for entry_id in entry_ids {
                for row in statement
                    .query_map([entry_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                    })
                    .map_err(|error| error.to_string())?
                {
                    let (id, image) = row.map_err(|error| error.to_string())?;
                    removed.insert(id);
                    if let Some(path) = image {
                        affected.insert(PathBuf::from(path));
                    }
                }
            }
        }
        let image_ownership = _image_ownership(&transaction, affected)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(ClipboardChange {
            removed,
            image_ownership,
        })
    }
}

fn write_entry(connection: &Connection, entry: &ClipboardEntry) -> Result<(), String> {
    let encoded = encode_content(&entry.content)?;
    connection
        .execute(
            "INSERT INTO clipboard_entries (
                    entry_id, content_kind, title, text_payload, files_json,
                    image_path, byte_size, captured_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(entry_id) DO UPDATE SET
                    content_kind = excluded.content_kind,
                    title = excluded.title,
                    text_payload = excluded.text_payload,
                    files_json = excluded.files_json,
                    image_path = excluded.image_path,
                    byte_size = excluded.byte_size,
                    captured_at = excluded.captured_at",
            params![
                entry.entry_id,
                encoded.kind,
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

fn prune(
    connection: &Connection,
    now: u64,
    config: &ClipboardConfig,
    affected: &mut HashSet<PathBuf>,
) -> Result<HashSet<String>, String> {
    let mut removed = HashSet::new();
    if let Some(cutoff) = config.cutoff_millis(now) {
        let mut statement = connection
            .prepare("DELETE FROM clipboard_entries WHERE captured_at < ?1 RETURNING entry_id, image_path")
            .map_err(|error| error.to_string())?;
        for row in statement
            .query_map([integer(cutoff)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|error| error.to_string())?
        {
            let (id, image) = row.map_err(|error| error.to_string())?;
            removed.insert(id);
            if let Some(path) = image {
                affected.insert(PathBuf::from(path));
            }
        }
    }
    if let Some(maximum) = config.max_entries {
        let mut statement = connection.prepare(
            "DELETE FROM clipboard_entries WHERE entry_id IN (
                SELECT entry_id FROM clipboard_entries ORDER BY captured_at DESC, entry_id LIMIT -1 OFFSET ?1
            ) RETURNING entry_id, image_path",
        ).map_err(|error| error.to_string())?;
        for row in statement
            .query_map([i64::from(maximum)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|error| error.to_string())?
        {
            let (id, image) = row.map_err(|error| error.to_string())?;
            removed.insert(id);
            if let Some(path) = image {
                affected.insert(PathBuf::from(path));
            }
        }
    }
    Ok(removed)
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

fn _image_ownership(
    connection: &Connection,
    affected: HashSet<PathBuf>,
) -> Result<std::collections::HashMap<PathBuf, bool>, String> {
    let mut ownership = std::collections::HashMap::with_capacity(affected.len());
    if affected.is_empty() {
        return Ok(ownership);
    }
    let mut statement = connection.prepare("SELECT EXISTS(SELECT 1 FROM clipboard_entries WHERE content_kind = 'image' AND image_path IS NOT NULL AND image_path = ?1)").map_err(|e| e.to_string())?;
    for path in affected {
        let retained = statement
            .query_row([path.to_string_lossy().as_ref()], |row| {
                row.get::<_, bool>(0)
            })
            .map_err(|e| e.to_string())?;
        ownership.insert(path, retained);
    }
    Ok(ownership)
}

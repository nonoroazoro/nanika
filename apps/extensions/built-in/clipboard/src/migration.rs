use rusqlite::{Connection, TransactionBehavior, params};
use std::path::Path;

const LEGACY_SCHEMA: &str = "
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

/// Only the owned storage worker calls this in production, before retention or payload cleanup.
pub(crate) fn open(path: &Path, payload_root: &Path) -> Result<Connection, String> {
    match nanika_database::open(path, crate::clipboard_database::SCHEMA) {
        Ok(connection) => Ok(connection),
        Err(rusqlite::Error::SqliteFailure(error, _))
            if error.extended_code == rusqlite::ffi::SQLITE_SCHEMA =>
        {
            let mut connection =
                nanika_database::open(path, LEGACY_SCHEMA).map_err(|e| e.to_string())?;
            _migrate(&mut connection, payload_root).map_err(|error| format!(
                "Clipboard metadata migration failed. Existing history was preserved. Restore the reported payload or resolve the database error, then restart the extension to retry: {error}"
            ))?;
            drop(connection);
            nanika_database::open(path, crate::clipboard_database::SCHEMA)
                .map_err(|e| e.to_string())
        }
        Err(error) => Err(error.to_string())
    }
}

fn _migrate(connection: &mut Connection, payload_root: &Path) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    // Recheck under the write lock: another owner may have migrated after admission.
    let expected = Connection::open_in_memory().map_err(|e| e.to_string())?;
    expected
        .execute_batch(LEGACY_SCHEMA)
        .map_err(|e| e.to_string())?;
    if _schema(&transaction)? != _schema(&expected)? {
        return Err("clipboard schema changed before migration; retry opening history".into());
    }
    transaction.execute_batch("DROP INDEX clipboard_entries_ordering; DROP INDEX clipboard_entries_images; ALTER TABLE clipboard_entries RENAME TO legacy_clipboard_entries;").map_err(|e| e.to_string())?;
    transaction
        .execute_batch(crate::clipboard_database::SCHEMA)
        .map_err(|e| e.to_string())?;
    {
        let mut statement = transaction.prepare("SELECT entry_id, content_kind, text_payload, image_path FROM legacy_clipboard_entries ORDER BY entry_id").map_err(|e| e.to_string())?;
        let mut rows = statement.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let id: String = row.get(0).map_err(|e| e.to_string())?;
            let kind: String = row.get(1).map_err(|e| e.to_string())?;
            let preview = row
                .get::<_, Option<String>>(2)
                .map_err(|e| e.to_string())?
                .as_deref()
                .map(crate::labels::text_preview);
            let dimensions = if kind == "image" {
                let path: String = row.get(3).map_err(|e| e.to_string())?;
                let bytes = nanika_platform::read_png_resource(Path::new(&path), payload_root)
                    .map_err(|e| format!("entry {id}, payload {path}: {e}"))?;
                let (width, height) = nanika_platform::png_resource_dimensions(&bytes)
                    .map_err(|e| format!("entry {id}, payload {path}: {e}"))?;
                Some((width, height))
            } else {
                None
            };
            // Copy the stored payload verbatim, including JSON whitespace, sizes and timestamps.
            transaction.execute("INSERT INTO clipboard_entries (entry_id, content_kind, text_preview, image_width, image_height, text_payload, files_json, image_path, byte_size, captured_at) SELECT entry_id, content_kind, ?2, ?3, ?4, text_payload, files_json, image_path, byte_size, captured_at FROM legacy_clipboard_entries WHERE entry_id = ?1", params![id, preview, dimensions.map(|d| d.0), dimensions.map(|d| d.1)]).map_err(|e| format!("entry {id}: {e}"))?;
        }
    }
    transaction
        .execute_batch("DROP TABLE legacy_clipboard_entries;")
        .map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())
}

fn _schema(connection: &Connection) -> Result<(u32, Vec<String>), String> {
    let version = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let objects = connection.prepare("SELECT json_array(type, name, tbl_name, sql) FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*' ORDER BY type, name").map_err(|e| e.to_string())?.query_map([], |row| row.get(0)).map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    Ok((version, objects))
}

#[cfg(test)]
#[path = "../tests/migration.rs"]
mod tests;

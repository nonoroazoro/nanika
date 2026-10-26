use rusqlite::{Connection, TransactionBehavior};
use std::path::Path;

const LEGACY_SCHEMA: &str = "
CREATE TABLE app_sources (
    source_id TEXT NOT NULL CHECK (source_id <> ''),
    entry_id TEXT NOT NULL CHECK (entry_id <> ''),
    source_key TEXT NOT NULL CHECK (source_key <> ''),
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    normalized_tokens TEXT NOT NULL,
    launch_kind TEXT NOT NULL CHECK (launch_kind IN ('macos-bundle', 'windows-shell-link', 'windows-packaged', 'executable')),
    target_path TEXT NOT NULL CHECK (target_path <> ''),
    arguments_json TEXT NOT NULL CHECK (arguments_json <> ''),
    icon_key TEXT NOT NULL,
    icon_source TEXT,
    priority INTEGER NOT NULL CHECK (priority >= 0),
    PRIMARY KEY (source_id, entry_id)
) STRICT;
PRAGMA user_version=1;
";

/// The discovery owner upgrades cached metadata without deleting entries or changing their IDs.
pub(crate) fn open(path: &Path) -> rusqlite::Result<Connection> {
    match nanika_database::open(path, crate::application_database::SCHEMA) {
        Ok(connection) => Ok(connection),
        Err(rusqlite::Error::SqliteFailure(error, _))
            if error.extended_code == rusqlite::ffi::SQLITE_SCHEMA =>
        {
            let mut connection = nanika_database::open(path, LEGACY_SCHEMA)?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let expected = Connection::open_in_memory()?;
            expected.execute_batch(LEGACY_SCHEMA)?;
            if _schema(&transaction)? != _schema(&expected)? {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_SCHEMA),
                    Some("application schema changed before migration; retry opening".into())
                ));
            }
            transaction.execute_batch("ALTER TABLE app_sources RENAME TO legacy_app_sources;")?;
            transaction.execute_batch(crate::application_database::SCHEMA)?;
            transaction.execute_batch("INSERT INTO app_sources (source_id, entry_id, source_key, display_name, normalized_tokens, launch_kind, target_path, arguments_json, icon_key, icon_source, priority, localized_names) SELECT source_id, entry_id, source_key, display_name, normalized_tokens, launch_kind, target_path, arguments_json, icon_key, icon_source, priority, '{}' FROM legacy_app_sources; DROP TABLE legacy_app_sources;")?;
            transaction.commit()?;
            drop(connection);
            nanika_database::open(path, crate::application_database::SCHEMA)
        }
        Err(error) => Err(error)
    }
}

fn _schema(connection: &Connection) -> rusqlite::Result<Vec<String>> {
    connection.prepare("SELECT json_array(type, name, tbl_name, sql) FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*' ORDER BY type, name")?
        .query_map([], |row| row.get(0))?.collect()
}

#[cfg(test)]
#[path = "../tests/migration.rs"]
mod tests;

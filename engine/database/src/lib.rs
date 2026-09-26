//! SQLite connection and exact-schema admission shared by independently owned databases.
#![forbid(unsafe_code)]

use rusqlite::{Connection, Result};
use std::path::Path;

/// Open only the current schema. Owners choose transactions and handle failures.
/// Existing mismatched databases are rejected without migration, repair, or deletion.
pub fn open(path: impl AsRef<Path>, schema: &str) -> Result<Connection> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    }
    let mut connection = Connection::open(path)?;
    let expected = Connection::open_in_memory()?;
    expected.execute_batch(schema)?;
    let expected_version: u32 = expected.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let objects = _objects(&connection)?;
    let initialize = objects.is_empty() && version == 0;
    if !initialize && (version != expected_version || objects != _objects(&expected)?) {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_SCHEMA),
            Some("unsupported database schema; existing data was not modified".to_owned()),
        ));
    }
    connection.execute_batch(
        "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
    )?;
    if initialize {
        let transaction = connection.transaction()?;
        transaction.execute_batch(schema)?;
        transaction.commit()?;
    }
    Ok(connection)
}

fn _objects(connection: &Connection) -> Result<Vec<(String, String, String, String)>> {
    connection.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*' ORDER BY type, name",
    )?.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?.collect()
}

#[cfg(test)]
#[path = "../tests/database.rs"]
mod tests;

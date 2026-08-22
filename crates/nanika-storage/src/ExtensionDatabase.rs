use std::fs;

use rusqlite::{Connection, Result as SqlResult};

use crate::{NanikaPaths, extension_id::is_valid_extension_id};

/// Extension-owned SQLite database with an isolated schema migration table.
pub struct ExtensionDatabase {
    connection: Connection,
}

impl ExtensionDatabase {
    pub fn open(paths: &NanikaPaths, extension_id: &str) -> SqlResult<Self> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        let directory = paths.database_dir().join("extensions");
        fs::create_dir_all(&directory)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let connection = Connection::open(directory.join(format!("{extension_id}.db")))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )?;
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> SqlResult<i64> {
        self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }
}

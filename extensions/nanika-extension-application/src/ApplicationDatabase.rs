use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::migrations::MIGRATIONS;
use crate::{ApplicationEntry, ApplicationError, ScanReport};

/// Application extension database owned by the discovery thread.
pub struct ApplicationDatabase {
    connection: Connection,
}

impl ApplicationDatabase {
    pub fn open_recovering(path: impl AsRef<Path>) -> Result<Self, ApplicationError> {
        let path = path.as_ref();
        match Self::open(path) {
            Ok(database) => match database.is_healthy() {
                Ok(true) => Ok(database),
                Ok(false) => {
                    drop(database);
                    Self::rebuild(path)
                }
                Err(error) if error.is_corrupt_database() => {
                    drop(database);
                    Self::rebuild(path)
                }
                Err(error) => Err(error),
            },
            Err(error) if error.is_corrupt_database() => Self::rebuild(path),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn rebuild(path: impl AsRef<Path>) -> Result<Self, ApplicationError> {
        let path = path.as_ref();
        remove_database_files(path)?;
        Self::open(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, ApplicationError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )?;
        apply_migrations(&mut connection)?;
        connection.execute(
            "UPDATE scan_state SET status = 'interrupted', completed_at = ?1, last_error = 'previous scan was interrupted' WHERE id = 1 AND status = 'running'",
            [timestamp_i64()],
        )?;
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> Result<i64, ApplicationError> {
        Ok(self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?)
    }

    fn is_healthy(&self) -> Result<bool, ApplicationError> {
        let result = self
            .connection
            .query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0))?;
        Ok(result == "ok")
    }

    pub fn load_active_entries(&self) -> Result<Vec<ApplicationEntry>, ApplicationError> {
        let mut statement = self.connection.prepare(
            "SELECT entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key, file_identity, last_seen_at, stale FROM app_entries WHERE stale = 0 ORDER BY normalized_name, entry_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ApplicationEntry {
                entry_id: row.get(0)?,
                source_key: row.get(1)?,
                display_name: row.get(2)?,
                normalized_name: row.get(3)?,
                normalized_tokens: row.get(4)?,
                launch_kind: row.get(5)?,
                target_path: row.get(6)?,
                working_directory: row.get(7)?,
                arguments_json: row.get(8)?,
                bundle_id: row.get(9)?,
                icon_key: row.get(10)?,
                file_identity: row.get(11)?,
                last_seen_at: u64::try_from(row.get::<_, i64>(12)?).unwrap_or(0),
                stale: row.get(13)?,
                icon_source: None,
                icon_index: 0,
                priority: 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn begin_scan(&self, generation: u64) -> Result<(), ApplicationError> {
        self.connection.execute(
            "UPDATE scan_state SET generation = ?1, status = 'running', started_at = ?2, completed_at = NULL, last_error = NULL WHERE id = 1",
            params![integer(generation), timestamp_i64()],
        )?;
        Ok(())
    }

    pub fn fail_scan(&self, generation: u64, error: &str) -> Result<(), ApplicationError> {
        self.connection.execute(
            "UPDATE scan_state SET status = 'failed', completed_at = ?1, last_error = ?2 WHERE id = 1 AND generation = ?3",
            params![timestamp_i64(), error, integer(generation)],
        )?;
        Ok(())
    }

    pub fn commit_scan(
        &mut self,
        report: ScanReport,
        entries: &[ApplicationEntry],
        last_error: Option<&str>,
    ) -> Result<(), ApplicationError> {
        let transaction = self.connection.transaction()?;
        if report.complete {
            transaction.execute("DELETE FROM app_entries WHERE stale = 1", [])?;
            transaction.execute("UPDATE app_entries SET stale = 1", [])?;
        }
        {
            let mut statement = transaction.prepare(
                "INSERT INTO app_entries (entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key, file_identity, last_seen_at, stale)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0)
                 ON CONFLICT(entry_id) DO UPDATE SET source_key = excluded.source_key, display_name = excluded.display_name, normalized_name = excluded.normalized_name, normalized_tokens = excluded.normalized_tokens, launch_kind = excluded.launch_kind, target_path = excluded.target_path, working_directory = excluded.working_directory, arguments_json = excluded.arguments_json, bundle_id = excluded.bundle_id, icon_key = excluded.icon_key, file_identity = excluded.file_identity, last_seen_at = excluded.last_seen_at, stale = 0",
            )?;
            for entry in entries {
                statement.execute(params![
                    entry.entry_id,
                    entry.source_key,
                    entry.display_name,
                    entry.normalized_name,
                    entry.normalized_tokens,
                    entry.launch_kind,
                    entry.target_path,
                    entry.working_directory,
                    entry.arguments_json,
                    entry.bundle_id,
                    entry.icon_key,
                    entry.file_identity,
                    integer(entry.last_seen_at),
                ])?;
            }
        }
        let status = if report.cancelled {
            "cancelled"
        } else if report.complete {
            "complete"
        } else {
            "partial"
        };
        transaction.execute(
            "UPDATE scan_state SET status = ?1, completed_at = ?2, last_error = ?3 WHERE id = 1 AND generation = ?4",
            params![status, timestamp_i64(), last_error, integer(report.generation)],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn scan_status(&self) -> Result<String, ApplicationError> {
        Ok(self
            .connection
            .query_row("SELECT status FROM scan_state WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?
            .unwrap_or_else(|| "missing".to_owned()))
    }
}

fn remove_database_files(path: &Path) -> Result<(), ApplicationError> {
    for candidate in [
        path.to_path_buf(),
        database_sidecar(path, "-wal"),
        database_sidecar(path, "-shm"),
    ] {
        if let Err(error) = std::fs::remove_file(candidate)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
    }
    Ok(())
}

fn database_sidecar(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    sidecar.into()
}

fn apply_migrations(connection: &mut Connection) -> Result<(), ApplicationError> {
    let applied = {
        let mut statement =
            connection.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
        statement
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    for (index, version) in applied.iter().enumerate() {
        let expected = i64::try_from(index + 1).unwrap_or(i64::MAX);
        if *version != expected {
            return Err(ApplicationError::Configuration(
                "application database has a non-contiguous migration history".to_owned(),
            ));
        }
    }
    let latest = MIGRATIONS.last().map_or(0, |(version, _)| *version);
    if applied.last().is_some_and(|version| *version > latest) {
        return Err(ApplicationError::Configuration(
            "application database was created by a newer extension".to_owned(),
        ));
    }
    for (version, sql) in MIGRATIONS.iter().skip(applied.len()).copied() {
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![version, timestamp_i64()],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

fn timestamp_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        })
}

fn integer(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

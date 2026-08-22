use std::fs;
use std::path::Path;

use nanika_search::{MAX_USAGE_ROWS, USAGE_RETENTION_DAYS};
use rusqlite::{Connection, Result as SqlResult, params};

use crate::{
    ExtensionKind, StoredUsage, extension_id::is_valid_extension_id, migrations::MIGRATIONS,
};

/// Host-owned SQLite database with embedded forward-only migrations.
pub struct HostDatabase {
    connection: Connection,
}

impl HostDatabase {
    pub fn open(path: impl AsRef<Path>) -> SqlResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )?;
        let current = validate_migration_history(&connection)?;
        for (version, sql) in MIGRATIONS
            .iter()
            .copied()
            .filter(|(version, _)| *version > current)
        {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, unixepoch())",
                params![version],
            )?;
            transaction.commit()?;
        }
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> SqlResult<i64> {
        self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }

    pub fn load_input_history(&self, limit: usize) -> SqlResult<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT display_query FROM input_history ORDER BY last_used_at DESC, id DESC LIMIT ?1",
        )?;
        let mut entries = statement
            .query_map(params![i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
                row.get(0)
            })?
            .collect::<SqlResult<Vec<String>>>()?;
        entries.reverse();
        Ok(entries)
    }

    pub fn load_usage(&self) -> SqlResult<Vec<StoredUsage>> {
        let mut statement = self.connection.prepare(
            "SELECT extension_id, entry_id, action_id, query_context, execution_count, last_executed_at
             FROM usage_stats",
        )?;
        statement
            .query_map([], |row| {
                Ok(StoredUsage {
                    extension_id: row.get(0)?,
                    entry_id: row.get(1)?,
                    action_id: row.get(2)?,
                    query_context: row.get(3)?,
                    execution_count: u32::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                    last_executed_at: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
                })
            })?
            .collect()
    }

    pub(crate) fn record_input_history(
        &self,
        history_key: &str,
        display_query: &str,
        used_at: u64,
        limit: usize,
    ) -> SqlResult<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO input_history (
                normalized_query, display_query, use_count, first_used_at, last_used_at
             ) VALUES (?1, ?2, 1, ?3, ?3)
             ON CONFLICT(normalized_query) DO UPDATE SET
                display_query = excluded.display_query,
                use_count = input_history.use_count + 1,
                last_used_at = excluded.last_used_at",
            params![
                history_key,
                display_query,
                i64::try_from(used_at).unwrap_or(i64::MAX),
            ],
        )?;
        transaction.execute(
            "DELETE FROM input_history WHERE id NOT IN (
                SELECT id FROM input_history ORDER BY last_used_at DESC, id DESC LIMIT ?1
             )",
            params![i64::try_from(limit).unwrap_or(i64::MAX)],
        )?;
        transaction.commit()
    }

    pub(crate) fn record_usage(
        &self,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
        executed_at: u64,
    ) -> SqlResult<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO usage_stats (
                extension_id, entry_id, action_id, query_context, execution_count, last_executed_at
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5)
             ON CONFLICT(extension_id, entry_id, action_id, query_context) DO UPDATE SET
                execution_count = MIN(usage_stats.execution_count + 1, 100),
                last_executed_at = excluded.last_executed_at",
            params![
                extension_id,
                entry_id,
                action_id,
                query_context,
                i64::try_from(executed_at).unwrap_or(i64::MAX),
            ],
        )?;
        let cutoff = executed_at.saturating_sub(USAGE_RETENTION_DAYS.saturating_mul(86_400));
        transaction.execute(
            "DELETE FROM usage_stats WHERE last_executed_at < ?1",
            params![i64::try_from(cutoff).unwrap_or(i64::MAX)],
        )?;
        transaction.execute(
            "DELETE FROM usage_stats WHERE rowid IN (
                SELECT rowid FROM usage_stats
                ORDER BY last_executed_at DESC, rowid DESC
                LIMIT -1 OFFSET ?1
             )",
            params![i64::try_from(MAX_USAGE_ROWS).unwrap_or(i64::MAX)],
        )?;
        transaction.commit()
    }

    pub(crate) fn register_extension(
        &self,
        extension_id: &str,
        kind: ExtensionKind,
        updated_at: u64,
    ) -> SqlResult<()> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        self.connection.execute(
            "INSERT INTO extensions (
                extension_id, kind, state, health, updated_at
             ) VALUES (?1, ?2, 'enabled', 'healthy', ?3)
             ON CONFLICT(extension_id) DO UPDATE SET
                kind = excluded.kind,
                state = 'enabled',
                health = 'healthy',
                updated_at = excluded.updated_at",
            params![
                extension_id,
                kind.as_str(),
                i64::try_from(updated_at).unwrap_or(i64::MAX),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn prune_usage(&self, now: u64) -> SqlResult<()> {
        let cutoff = now.saturating_sub(USAGE_RETENTION_DAYS.saturating_mul(86_400));
        self.connection.execute(
            "DELETE FROM usage_stats WHERE last_executed_at < ?1",
            params![i64::try_from(cutoff).unwrap_or(i64::MAX)],
        )?;
        self.connection.execute(
            "DELETE FROM usage_stats WHERE rowid IN (
                SELECT rowid FROM usage_stats
                ORDER BY last_executed_at DESC, rowid DESC
                LIMIT -1 OFFSET ?1
             )",
            params![i64::try_from(MAX_USAGE_ROWS).unwrap_or(i64::MAX)],
        )?;
        Ok(())
    }

    pub(crate) fn reset_usage(&self) -> SqlResult<()> {
        self.connection.execute("DELETE FROM usage_stats", [])?;
        Ok(())
    }
}

fn validate_migration_history(connection: &Connection) -> SqlResult<i64> {
    let mut statement =
        connection.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
    let applied = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<SqlResult<Vec<_>>>()?;
    if applied.len() > MIGRATIONS.len() {
        return Err(rusqlite::Error::InvalidParameterName(
            "database schema is newer than this host".to_owned(),
        ));
    }
    for (index, version) in applied.iter().enumerate() {
        if *version != MIGRATIONS[index].0 {
            return Err(rusqlite::Error::InvalidParameterName(
                "database migration history is not contiguous".to_owned(),
            ));
        }
    }
    Ok(applied.last().copied().unwrap_or(0))
}

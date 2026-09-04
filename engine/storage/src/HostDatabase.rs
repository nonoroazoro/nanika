use std::fs;
use std::path::Path;

use nanika_search::{MAX_USAGE_ROWS, USAGE_RETENTION_DAYS};
use rusqlite::{Connection, OptionalExtension, Result as SqlResult, params};

use crate::{
    ExtensionKind, StoredExtension, StoredExtensionLoad, StoredUsage,
    extension_id::is_valid_extension_id,
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS extensions (
    extension_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    installed_version TEXT,
    active_version TEXT,
    install_path TEXT,
    package_digest TEXT,
    state TEXT NOT NULL,
    health TEXT NOT NULL,
    last_error TEXT,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS input_history (
    id INTEGER PRIMARY KEY,
    normalized_query TEXT NOT NULL UNIQUE,
    display_query TEXT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 0,
    first_used_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS usage_stats (
    extension_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    query_context TEXT NOT NULL,
    execution_count INTEGER NOT NULL DEFAULT 0,
    last_executed_at INTEGER NOT NULL,
    PRIMARY KEY (extension_id, entry_id, action_id, query_context),
    FOREIGN KEY (extension_id) REFERENCES extensions(extension_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS usage_stats_last_executed_at
ON usage_stats(last_executed_at DESC);
";

/// Host-owned SQLite database using the current pre-release schema baseline.
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
        connection.execute_batch(SCHEMA)?;
        Ok(Self { connection })
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

    pub fn load_extensions(&self) -> SqlResult<Vec<StoredExtension>> {
        Ok(self.load_extensions_isolated()?.extensions)
    }

    pub fn load_extensions_isolated(&self) -> SqlResult<StoredExtensionLoad> {
        let mut statement = self.connection.prepare(
            "SELECT extension_id, kind, installed_version, active_version, install_path,
                    package_digest, state, health, last_error
             FROM extensions
             ORDER BY extension_id",
        )?;
        let mut rows = statement.query([])?;
        let mut extensions = Vec::new();
        let mut errors = Vec::new();
        let mut row_number = 0_usize;
        while let Some(row) = rows.next()? {
            row_number += 1;
            let identity = row
                .get_ref(0)
                .ok()
                .and_then(|value| value.as_str().ok())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("row {row_number}"));
            match stored_extension_from_row(row) {
                Ok(extension) => match validate_stored_extension_metadata(&extension) {
                    Ok(()) => extensions.push(extension),
                    Err(error) => errors.push(format!(
                        "extension {identity} has invalid metadata ({error}) and was skipped"
                    )),
                },
                Err(error) => errors.push(format!(
                    "extension {identity} has invalid metadata ({error}) and was skipped"
                )),
            }
        }
        Ok(StoredExtensionLoad { extensions, errors })
    }

    pub fn extension(&self, extension_id: &str) -> SqlResult<Option<StoredExtension>> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        self.connection
            .query_row(
                "SELECT extension_id, kind, installed_version, active_version, install_path,
                        package_digest, state, health, last_error
                 FROM extensions WHERE extension_id = ?1",
                params![extension_id],
                stored_extension_from_row,
            )
            .optional()
    }

    pub fn install_external_extension(
        &self,
        extension_id: &str,
        version: &str,
        install_path: &Path,
        package_digest: &str,
        enabled: bool,
        updated_at: u64,
    ) -> SqlResult<()> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        self.connection.execute(
            "INSERT INTO extensions (
                extension_id, kind, installed_version, active_version, install_path,
                package_digest, state, health, last_error, updated_at
             ) VALUES (?1, 'external', ?2, ?2, ?3, ?4, ?5, 'healthy', NULL, ?6)
             ON CONFLICT(extension_id) DO UPDATE SET
                installed_version = excluded.installed_version,
                active_version = excluded.active_version,
                install_path = excluded.install_path,
                package_digest = excluded.package_digest,
                state = excluded.state,
                health = 'healthy',
                last_error = NULL,
                updated_at = excluded.updated_at
             WHERE extensions.kind = 'external'",
            params![
                extension_id,
                version,
                install_path.to_string_lossy(),
                package_digest,
                if enabled { "enabled" } else { "disabled" },
                i64::try_from(updated_at).unwrap_or(i64::MAX),
            ],
        )?;
        Ok(())
    }

    pub fn set_external_extension_enabled(
        &self,
        extension_id: &str,
        enabled: bool,
        updated_at: u64,
    ) -> SqlResult<bool> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        let changed = self.connection.execute(
            "UPDATE extensions SET state = ?2, updated_at = ?3
             WHERE extension_id = ?1 AND kind = 'external'",
            params![
                extension_id,
                if enabled { "enabled" } else { "disabled" },
                i64::try_from(updated_at).unwrap_or(i64::MAX),
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn remove_external_extension(&self, extension_id: &str) -> SqlResult<bool> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        Ok(self.connection.execute(
            "DELETE FROM extensions WHERE extension_id = ?1 AND kind = 'external'",
            params![extension_id],
        )? == 1)
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

fn stored_extension_from_row(row: &rusqlite::Row<'_>) -> SqlResult<StoredExtension> {
    let kind = row.get::<_, String>(1)?;
    let kind = ExtensionKind::parse(&kind).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(1, "kind".to_owned(), rusqlite::types::Type::Text)
    })?;
    Ok(StoredExtension {
        extension_id: row.get(0)?,
        kind,
        installed_version: row.get(2)?,
        active_version: row.get(3)?,
        install_path: row.get::<_, Option<String>>(4)?.map(Into::into),
        package_digest: row.get(5)?,
        state: row.get(6)?,
        health: row.get(7)?,
        last_error: row.get(8)?,
    })
}

fn validate_stored_extension_metadata(extension: &StoredExtension) -> Result<(), &'static str> {
    if !is_valid_extension_id(&extension.extension_id) {
        return Err("invalid extension id");
    }
    if !matches!(extension.state.as_str(), "enabled" | "disabled") {
        return Err("invalid extension state");
    }
    if extension.health.trim().is_empty() {
        return Err("empty extension health");
    }
    Ok(())
}
